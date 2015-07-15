use std::fs::File;
use std::io::{BufReader, BufRead, Write};
use std::collections::{HashMap};

use std::env;
use std::io;

const WHITESPACE_FACTOR: isize = 5;
const WHITESPACE_REDUCE: isize = 2;
const CLASS_FACTOR: isize = 5;
const CLASS_REDUCE: isize = 2;

#[derive(Debug)]
struct LineInfo {
    line: String,
    char_map: HashMap<char, Vec<usize>>,
    heatmap: Vec<isize>
}

#[derive(PartialEq)]
enum CharClass {
    Whitespace,
    Numeric,
    Alphabetic,
    Other
}

impl<T: Into<String>> From<T> for LineInfo {
    fn from(item: T) -> LineInfo {
        let mut map: HashMap<char, Vec<usize>> = HashMap::new();
        let mut heat = vec![];
        let line = item.into();

        let mut ws_score = 0;
        let mut cs_score = 0;
        let mut cur_class = CharClass::Whitespace;

        for (idx, c) in line.chars().enumerate() {
            // don't map whitespace
            if !c.is_whitespace() {
                // update the character class change score if needed
                if c.is_numeric() {
                    if cur_class != CharClass::Numeric {
                        cur_class = CharClass::Numeric;
                        cs_score += CLASS_FACTOR;
                    }
                } else if c.is_alphabetic() {
                    if cur_class != CharClass::Alphabetic {
                        cur_class = CharClass::Alphabetic;
                        cs_score += CLASS_FACTOR;
                    }
                } else {
                    if cur_class != CharClass::Other {
                        cur_class = CharClass::Other;
                        cs_score += CLASS_FACTOR;
                    }
                }

                // add an entry in the character map
                map.entry(c).or_insert(Vec::new()).push(idx);
                if c.is_uppercase() {
                    for lc in c.to_lowercase() {
                        // also insert all lowercase equivalents of this character
                        // but not the other way around, so that typing something
                        // uppercase specifies to match uppercase
                        map.entry(lc).or_insert(Vec::new()).push(idx);
                    }
                }
            } else {
                // whitespace is treated differently
                cur_class = CharClass::Whitespace;
                ws_score = WHITESPACE_FACTOR;
            }

            // push to the heatmap
            heat.push(ws_score + cs_score);

            // reduce things
            ws_score /= WHITESPACE_REDUCE;
            cs_score /= CLASS_REDUCE;
        }

        LineInfo {
            line: line,
            char_map: map,
            heatmap: heat
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

    let mut positions = vec![];

    for m_line in input_file.lines() {
        let line = match m_line {
            Ok(l) => l,
            Err(e) => panic!("Failed to read line: {}", e)
        };

        let info = LineInfo::from(line.clone());
        positions.clear();

        for c in query.chars() {
            match info.char_map.get(&c) {
                None => break,
                Some(list) => {
                    positions.push(list.clone());
                }
            }
        }

        if positions.len() == query.len() {
            // matching line
            println!("Line {:?}: positions {:?}", line, positions);
        }
    }
}
