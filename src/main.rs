#![feature(collections)]
#![feature(into_cow)]
#![feature(iter_arith)]

use std::io::Write;

use std::env;
use std::io;

use search::SearchBase;

mod search;
mod error;

fn main() {
    let history_path = match env::var("HISTFILE") {
        Ok(p) => p,
        Err(e) => panic!("Failed to get bash history file: {}", e)
    };

    // create a hashmap of lines to info
    let mut base = SearchBase::default();

    // read the history
    println!("Reading history...");
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
        Ok(_) => {}
    }

    match io::stdin().read_line(&mut query) {
        Ok(_) => {},
        Err(e) => panic!("Failed to read input line: {}", e)
    }

    match query.pop() {
        Some('\n') => {/* pop off trailing newline */},
        Some(c) => query.push(c),
        None => {/* Do nothing with an empty query */}
    }

    let result = base.query(&query);

    println!("Matches:");

    for item in result.iter() {
        println!("{}", item);
    }
}
