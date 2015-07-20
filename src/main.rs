#![feature(cstr_to_str)]
#![feature(collections)]
#![feature(into_cow)]
#![feature(iter_arith)]
extern crate libc;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate term;

use std::io::prelude::*;

use std::env;
use std::io;

use search::SearchBase;

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

    debug!("Getting history path");

    let history_path = match env::var("HISTFILE") {
        Ok(p) => {
            trace!("Got history path: {:?}", p);
            p
        },
        Err(e) => panic!("Failed to get bash history file: {}", e)
    };

    // create a hashmap of lines to info
    let mut base = SearchBase::default();

    // read the history
    info!("Reading history");
    match base.read_history(history_path) {
        Ok(_) => {
            // success
        },
        Err(e) => {
            panic!("Failed to read history: {}", e)
        }
    }

    let mut query = String::new();

    print!("Match input: ");
    match io::stdout().flush() {
        Err(e) => panic!("Failed to flush stdout: {}", e),
        Ok(_) => {
            trace!("Successfully flushed output");
        }
    }

    match io::stdin().read_line(&mut query) {
        Ok(_) => {
            trace!("Successfully read line");
        },
        Err(e) => panic!("Failed to read input line: {}", e)
    }

    debug!("Got query: {:?}", query);

    match query.pop() {
        Some('\n') => {/* pop off trailing newline */},
        Some(c) => query.push(c),
        None => {/* Do nothing with an empty query */}
    }

    debug!("Querying search base");

    let result = base.query(&query);

    println!("Matches:");

    for item in result.iter() {
        println!("{}", item);
    }
}
