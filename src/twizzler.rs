#![cfg(target_os = "twizzler")]
/// Twizzler virtual table implementation for rusqlite
/// Future work includes:
/// - Implementing the ability to add indexes and query optimization in best_index (Will likely require changes to the connection struct to intercept queries,
/// though I may be wrong)
/// - Filtering results before returning them to SQLite in filter for more constraints
/// - RowID column handling (ex WITHOUT ROWID tables, or queries based on RowID)

use crate::vtab::{update_module, CreateVTab, UpdateVTab, VTab, VTabCursor, VTabKind, IndexConstraintOp, Values};

use crate::Connection;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::fmt::Debug;

use naming::GetFlags;
use twizzler::{
    collections::{
        hachage::PersistentHashMap,
    },
    object::{Object, ObjectBuilder},
    marker::Invariant,
};
use twizzler_rt_abi::object::MapFlags;

mod value;
use value::TwzValue;
mod columnstore;
use columnstore::{ColumnStore, MAX_COLUMNS};

fn open_or_create_hashtable_object(
    name: &str,
) -> crate::Result<PersistentHashMap<i64, Row>> {
    let mut nh = naming::dynamic_naming_factory().unwrap();
    let name = format!("/data/vtab-{}", name);
    let vo = if let Ok(node) = nh.get(&name, GetFlags::empty()) {
        println!("reopened: {:?}", node.id);
        let backing = Object::map(node.id, MapFlags::PERSIST | MapFlags::READ | MapFlags::WRITE)
            .map_err(|e| crate::Error::ModuleError(format!("Failed to map object: {}", e)))?;
        let phm = PersistentHashMap::from(backing);
        Ok(phm)
    } else {
        let vo = PersistentHashMap::with_builder(
            ObjectBuilder::default().persist()
        ).unwrap();
        let _ = nh.remove(&name);
        nh.put(&name, vo.object().id())
            .map_err(|e| crate::Error::ModuleError(format!("Failed to put object in naming: {}", e)))?;
        Ok(vo)
    };

    vo
}

impl Connection {
    /// Sets up the Twizzler virtual table module for this connection.
        pub fn setup_twz_vtab(&self) {
            let module = update_module::<TwzVTab>();
            self.create_module("twz_vtab", module, None)
                .expect("Failed to create twz_vtab module");
        }
}

#[derive(Debug, Clone)]
#[repr(C)]
struct Row {
    id: i64,
    columns: ColumnStore,
}
unsafe impl Invariant for Row {}

struct DataStore {
    hm: PersistentHashMap<i64, Row>,
}
#[repr(C)]
struct TwzVTab {
    base: crate::ffi::sqlite3_vtab,
    data: Arc<RwLock<DataStore>>,
}

#[repr(C)]
struct TwzCursor {
    base: crate::ffi::sqlite3_vtab_cursor,
    position: i64,
    current_results: Vec<i64>,
    data: Arc<RwLock<DataStore>>,
    // constraints: Option<Values<'a>>,
}

struct TwzConfig;

unsafe impl<'vtab> VTab<'vtab> for TwzVTab {
    type Aux = TwzConfig;

    type Cursor = TwzCursor;

    fn connect(
        db: &mut crate::vtab::VTabConnection,
        aux: Option<&Self::Aux>,
        args: &[&[u8]],
    ) -> crate::Result<(String, Self)> {
        let mut columns = Vec::new();

        let mut args_iter = args.iter().skip(2); // Skip first two args (module name and db name)
        let table_name = String::from_utf8_lossy(args_iter.next().unwrap()).trim().to_string();

        for arg in args_iter {
            let arg_str = String::from_utf8_lossy(arg);
            let mut trimmed = arg_str.trim().to_string();

            if (trimmed.starts_with('\'') && trimmed.ends_with('\'')) ||
               (trimmed.starts_with('"') && trimmed.ends_with('"')) {
                trimmed = trimmed[1..trimmed.len()-1].to_string();
            }
            
            if trimmed.is_empty() {
                continue;
            }

            let (col_name, col_type) = if trimmed.contains(':') {
                let parts: Vec<&str> = trimmed.split(':').collect();
                if parts.len() == 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    continue;
                }
            } else {
                println!("Warning: Error parsing args: {}", trimmed);
                continue;
            };

            columns.push(format!("{} {}", col_name, col_type));
        }

        if columns.len() > MAX_COLUMNS {
            return Err(crate::Error::ModuleError(format!("Too many columns specified (max {})", MAX_COLUMNS)));
        }

        let schema = format!("CREATE TABLE {}({})", table_name, columns.join(", "));
        println!("TwzVTab schema: {}", schema);

        let vtab = TwzVTab {
            base: crate::ffi::sqlite3_vtab::default(),
            data: Arc::new(RwLock::new(DataStore {
                hm: open_or_create_hashtable_object(&table_name).map_err(|e| crate::Error::ModuleError(format!("Failed to open or create hashtable object: {:?}", e)))?,
            })),
        };

        Ok((schema, vtab))
    }

    fn best_index(&self, info: &mut crate::vtab::IndexInfo) -> crate::Result<()> {
        let mut constraints = info.constraints_and_usages();
        let mut argv_index = 1; // Any argv > 0 is passed to filter in filter()
        let mut idx_num = 0;
        for (constraint, mut usage) in constraints {
            // Example handling for constraints. More complex logic will be necessary here to handle multiple instances of the same constraint type,
            // as well as if there are more than one constraint. I don't think filter is passed any info to indicate which constraint is which in args.
            match constraint.operator() {
                IndexConstraintOp::SQLITE_INDEX_CONSTRAINT_EQ => {
                    // We can handle equality constraints.
                    usage.set_omit(false);
                    // ArgvIndex 
                    usage.set_argv_index(argv_index);
                    argv_index += 1;
                    // Set idx_num to indicate that we need to filter on this column.
                    idx_num |= 1 << 2; // Example: using bit 2 to indicate equality constraint
                }  
                _ => {
                    // We don't handle this constraint at the moment, so we can just leave everything as it is.
                }
            };
            usage.set_omit(false);
            // usage.set_argv_index(0);
        }
        info.set_estimated_cost(1000.0);
        info.set_estimated_rows(1000);
        Ok(())
    }

    fn open(&'vtab mut self) -> crate::Result<Self::Cursor> {
        Ok(TwzCursor {
            base: crate::ffi::sqlite3_vtab_cursor::default(),
            position: 0,
            current_results: Vec::new(),
            data: self.data.clone(),
            // constraints: None,
        })
    }
}

impl<'vtab> CreateVTab<'vtab> for TwzVTab {
    const KIND: VTabKind = VTabKind::Default;
    
    fn create(
        db: &mut crate::vtab::VTabConnection,
        aux: Option<&Self::Aux>,
        args: &[&[u8]],
    ) -> crate::Result<(String, Self)> {
        Self::connect(db, aux, args)
    }

    fn destroy(&self) -> crate::Result<()> {
        Ok(())
    }
}
impl<'vtab> UpdateVTab<'vtab> for TwzVTab {
    fn delete(&mut self, arg: crate::types::ValueRef<'_>) -> crate::Result<()> {
        let res = self.data.write().unwrap().hm.remove(&arg.as_i64().unwrap());
        match res {
            Some(_) => Ok(()),
            None => Err(crate::Error::ModuleError("Error in TwzVTab::delete".to_string())),
        }
    }

    fn insert(&mut self, args: &crate::vtab::Values<'_>) -> crate::Result<i64> {
        let mut data = self.data.write().unwrap();
        let mut args_iter = args.iter().skip(1);
        let row_id_ref = args_iter.next().unwrap();
        let row_id = match row_id_ref {
            crate::types::ValueRef::Integer(_) => row_id_ref.as_i64().unwrap(),
            _ => data.hm.len() as i64 + 1,
        };
        let column_values: Vec<TwzValue> = args.iter().skip(2).map(|v| v.into()).collect();
        let columns = ColumnStore::from_values(&column_values)
            .map_err(|e| crate::Error::ModuleError(format!("Failed to create ColumnStore: {}", e)))?;

        data.hm.insert(row_id, Row { id: row_id, columns });
        Ok(row_id)
    }

    fn update(&mut self, args: &crate::vtab::Values<'_>) -> crate::Result<()> {
        let mut data = self.data.write().unwrap();
        let mut args_iter = args.iter();

        // Get old and new rowids
        let old_rowid = args_iter.next().unwrap().as_i64().unwrap();
        let new_rowid = args_iter.next().unwrap().as_i64().unwrap();

        // Get the existing row
        let mut row = data.hm.remove(&old_rowid)
            .ok_or_else(|| crate::Error::ModuleError(format!("Row {} not found", old_rowid)))?;
        
        // Update the row data
        row.id = new_rowid;
        
        // Update column values (skip first two args which are rowids)
        if args.len() > 2 {
            let column_values: Vec<TwzValue> = args_iter.map(|v| v.into()).collect();
            row.columns = ColumnStore::from_values(&column_values)
                .map_err(|e| crate::Error::ModuleError(format!("Failed to create ColumnStore: {}", e)))?;
        }
        
        // Insert with new rowid (even if same as old)
        data.hm.insert(new_rowid, row);
        
        Ok(())
    }
}

unsafe impl VTabCursor for TwzCursor {
    // Index number and string are best_index implementation dependent, with args containing the values to compare against. 
    fn filter(&mut self, idx_num: std::os::raw::c_int, idx_str: Option<&str>, args: &crate::vtab::Values<'_>) -> crate::Result<()> {
        let data = self.data.read().unwrap();
        self.current_results = data.hm.keys().cloned().collect();
        self.position = 0;
        drop(data);
        Ok(())
    }

    fn next(&mut self) -> crate::Result<()> {
        self.position += 1;
        Ok(())
    }

    fn eof(&self) -> bool {
        self.position as usize >= self.current_results.len()
    }

    fn column(&self, ctx: &mut crate::vtab::Context, i: std::os::raw::c_int) -> crate::Result<()> {
        if self.eof() {
            return Ok(());
        }
        
        let current_idx = self.position as usize;
        if current_idx >= self.current_results.len() {
            return Ok(());
        }
        
        let row_id = self.current_results[current_idx];

        let data = self.data.read().unwrap();
        if let Some(row) = data.hm.get(&row_id) {
            if let Some(column_data) = row.columns.get_column(i as usize) {
                ctx.set_result(column_data)?;
            } else {
                ctx.set_result(&TwzValue::Null)?;
            }
        } else {
            ctx.set_result(&TwzValue::Null)?;
        }

        Ok(())
    }

    fn rowid(&self) -> crate::Result<i64> {
        if self.eof() {
            return Err(crate::Error::ModuleError("Cursor at EOF".to_string()));
        }
        
        let current_idx = self.position as usize;
        if current_idx < self.current_results.len() {
            Ok(self.current_results[current_idx])
        } else {
            Err(crate::Error::ModuleError("Invalid cursor position".to_string()))
        }
    }
}

