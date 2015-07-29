// Copyright 2015 Jerome Rasky <jerome@rasky.co>
//
// Licensed under the Apache License, version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at
//
//     <http://www.apache.org/licenses/LICENSE-2.0>
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either expressed or implied. See the
// License for the specific language concerning governing permissions and
// limitations under the License.
#![feature(cstr_to_str)]
#![feature(convert)]
#![feature(collections)]
#![feature(into_cow)]
#![feature(iter_arith)]
#![feature(io)]
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
mod constants;

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
