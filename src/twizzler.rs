use crate::vtab::{update_module, CreateVTab, UpdateVTab, VTab, VTabCursor, VTabKind};
use crate::Connection;

impl Connection {
    pub fn setup_twz_vtab(&self) {
        let module = update_module::<TwzVTab>();
        self.create_module("twz_vtab", module, None)
            .expect("Failed to create twz_vtab module");
    }
}

struct TwzVTab;
struct TwzCursor;

struct TwzConfig;

unsafe impl<'vtab> VTab<'vtab> for TwzVTab {
    type Aux = TwzConfig;

    type Cursor = TwzCursor;

    fn connect(
        db: &mut crate::vtab::VTabConnection,
        aux: Option<&Self::Aux>,
        args: &[&[u8]],
    ) -> crate::Result<(String, Self)> {
        todo!()
    }

    fn best_index(&self, info: &mut crate::vtab::IndexInfo) -> crate::Result<()> {
        todo!()
    }

    fn open(&'vtab mut self) -> crate::Result<Self::Cursor> {
        todo!()
    }
}

unsafe impl VTabCursor for TwzVTab {
    fn filter(&mut self, idx_num: std::os::raw::c_int, idx_str: Option<&str>, args: &crate::vtab::Values<'_>) -> crate::Result<()> {
        todo!()
    }

    fn next(&mut self) -> crate::Result<()> {
        todo!()
    }

    fn eof(&self) -> bool {
        todo!()
    }

    fn column(&self, ctx: &mut crate::vtab::Context, i: std::os::raw::c_int) -> crate::Result<()> {
        todo!()
    }

    fn rowid(&self) -> crate::Result<i64> {
        todo!()
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
        todo!()
    }

    fn insert(&mut self, args: &crate::vtab::Values<'_>) -> crate::Result<i64> {
        todo!()
    }

    fn update(&mut self, args: &crate::vtab::Values<'_>) -> crate::Result<()> {
        todo!()
    }
}

unsafe impl VTabCursor for TwzCursor {
    fn filter(&mut self, idx_num: std::os::raw::c_int, idx_str: Option<&str>, args: &crate::vtab::Values<'_>) -> crate::Result<()> {
        todo!()
    }

    fn next(&mut self) -> crate::Result<()> {
        todo!()
    }

    fn eof(&self) -> bool {
        todo!()
    }

    fn column(&self, ctx: &mut crate::vtab::Context, i: std::os::raw::c_int) -> crate::Result<()> {
        todo!()
    }

    fn rowid(&self) -> crate::Result<i64> {
        todo!()
    }
}