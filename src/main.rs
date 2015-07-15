use std::fs::File;
use std::io::{BufReader, BufRead, Write};
use std::collections::{HashMap};

use std::env;
use std::io;

#[derive(Debug)]
struct LineInfo {
    line: String,
    char_map: HashMap<char, Vec<usize>>
}

impl<T: Into<String>> From<T> for LineInfo {
    fn from(item: T) -> LineInfo {
        let mut map: HashMap<char, Vec<usize>> = HashMap::new();
        let line = item.into();

        for (idx, c) in line.chars().enumerate() {
            if !c.is_whitespace() {
                map.entry(c).or_insert(Vec::new()).push(idx);
                if c.is_uppercase() {
                    for lc in c.to_lowercase() {
                        // also insert all lowercase equivalents of this character
                        // but not the other way around, so that typing something
                        // uppercase specifies to match uppercase
                        map.entry(lc).or_insert(Vec::new()).push(idx);
                    }
                }
            }
        }

        LineInfo {
            line: line,
            char_map: map
        }
    }
}

fn main() {
    let history_path = match env::var("HISTFILE") {
        Ok(p) => p,
        Err(e) => panic!("Failed to get bash history file: {}", e)
    };

    let input_file = match File::open(&history_path) {
        Ok(f) => BufReader::new(f),
        Err(e) => panic!("Could not open history file: {}", e)
    };

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

    for m_line in input_file.lines() {
        let line = match m_line {
            Ok(l) => l,
            Err(e) => panic!("Failed to read line: {}", e)
        };

        println!("Map for line {:?}: {:?}", line.clone(), LineInfo::from(line));
    }
}
