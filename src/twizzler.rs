use crate::vtab::{update_module, CreateVTab, UpdateVTab, VTab, VTabCursor, VTabKind};
use crate::Connection;
use crate::types::Value;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

impl Connection {
    /// Sets up the Twizzler virtual table module for this connection.
        pub fn setup_twz_vtab(&self) {
            let module = update_module::<TwzVTab>();
            self.create_module("twz_vtab", module, None)
                .expect("Failed to create twz_vtab module");
        }
}

#[derive(Debug, Clone)]
struct Row {
    id: i64,
    columns: Vec<Value>,
}

struct DataStore {
    hm: HashMap<i64, Row>,
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

        let schema = format!("CREATE TABLE {}({})", table_name, columns.join(", "));
        println!("TwzVTab schema: {}", schema);

        let vtab = TwzVTab {
            base: crate::ffi::sqlite3_vtab::default(),
            data: Arc::new(RwLock::new(DataStore {
                hm: HashMap::new(),
            })),
        };

        Ok((schema, vtab))
    }

    fn best_index(&self, info: &mut crate::vtab::IndexInfo) -> crate::Result<()> {
        // let mut constraints = info.constraints_and_usages();
        // for (constraint, mut  usage) in constraints {
        //     usage.set_omit(true);
        //     usage.set_argv_index(0);
        // }
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
        let column_values: Vec<Value> = args.iter().skip(2).map(|v| v.into()).collect();
        data.hm.insert(row_id, Row { id: row_id, columns: column_values });
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
            row.columns = args_iter.map(|v| v.into()).collect();
        }
        
        // Insert with new rowid (even if same as old)
        data.hm.insert(new_rowid, row);
        
        Ok(())
    }
}

unsafe impl VTabCursor for TwzCursor {
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

        let result = {
            let data = self.data.read().unwrap();
            if let Some(row) = data.hm.get(&row_id) {
                let column_data = row.columns.get(i as usize).unwrap();
                ctx.set_result(column_data)?;
            } else {
                ctx.set_result(&Value::Null)?;
            }
        };

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