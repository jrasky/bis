#![feature(cstr_to_str)]
#![feature(collections)]
#![feature(into_cow)]
#![feature(iter_arith)]
#![feature(str_escape)]
#![feature(io)]
#![feature(convert)]
#![feature(mpsc_select)]
extern crate libc;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate term;
extern crate unicode_width;

use ui::UI;

mod search;
mod error;
mod bis_c;
mod ui;

fn main() {
    // init logging
    match env_logger::init() {
        Ok(()) => {
            trace!("Logging initialized successfully");
        },
        Err(e) => {
            panic!("Failed to initialize logging: {}", e);
        }
    }

    // create the UI instance
    debug!("Creating UI instance");

    let mut ui = match UI::create() {
        Err(e) => {
            panic!("Failed to create UI instance: {}", e);
        },
        Ok(ui) => {
            trace!("UI instance created successfully");
            ui
        }
    };

    // start the ui
    debug!("Starting UI");

    match ui.start() {
        Ok(_) => {
            debug!("UI finished successfully");
        },
        Err(e) => {
            panic!("UI failure: {}", e);
        }
    }
}
