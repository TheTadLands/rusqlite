#![cfg(target_os = "twizzler")]
use crate::vtab::{update_module, update_module_with_tx};
use crate::Connection;

mod value;
mod rowstore;

mod transient_vtab;
use transient_vtab::{TwzVTab as TransientVTab};

mod persistent_vtab;
use persistent_vtab::{TwzVTab as PersistentVTab};

impl Connection {
    /// Sets up the Twizzler virtual table module for this connection.
        pub fn setup_twz_vtab(&self) {
            let module = update_module::<TransientVTab>();
            self.create_module("twz_transient_vtab", module, None)
                .expect("Failed to create twz_transient_vtab module");

            let module = update_module_with_tx::<PersistentVTab>();
            self.create_module("twz_persistent_vtab", module, None)
                .expect("Failed to create twz_persistent_vtab module");
        }
}

