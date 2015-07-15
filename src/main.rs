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

impl LineInfo {
    fn query_sequence<T: AsRef<str>>(&self, query_item: T) -> Option<Vec<Vec<usize>>> {
        let query = query_item.as_ref();
        let mut positions: Vec<Vec<usize>> = vec![];

        for c in query.chars() {
            match self.char_map.get(&c) {
                None => break,
                Some(list) => {
                    let to_push;
                    match positions.last() {
                        None => {
                            to_push = list.clone();
                        },
                        Some(item) => {
                            match list.binary_search(&item[0]) {
                                Ok(idx) => {
                                    if idx >= list.len() - 1 {
                                        // line is non-matching
                                        break;
                                    } else {
                                        to_push = list.split_at(idx + 1).1.into();
                                    }
                                },
                                Err(idx) => {
                                    if idx >= list.len() {
                                        // line is non-matching
                                        break;
                                    } else {
                                        to_push = list.split_at(idx).1.into();
                                    }
                                }
                            }
                        }
                    }
                    positions.push(to_push);
                }
            }
        }

        if positions.len() == query.len() {
            Some(positions)
        } else {
            None
        }
    }

    fn query_positions<T: AsRef<str>>(&self, query: T) -> Option<Vec<Vec<usize>>> {
        match self.query_sequence(query) {
            None => None,
            Some(positions) => {
                // matching line
                // create our idx vector
                let mut idx = vec![0; positions.len()];
                let mut result = vec![];
                loop {
                    // check that current configuration is strictly increasing
                    let mut ignore = false;
                    {
                        let mut last_pos = None;
                        for (i, pos) in idx.iter().enumerate() {
                            match last_pos {
                                None => last_pos = Some(positions[i][*pos]),
                                Some(other) => {
                                    if other >= positions[i][*pos] {
                                        ignore = true;
                                        break;
                                    } else {
                                        last_pos = Some(positions[i][*pos]);
                                    }
                                }
                            }
                        }
                    }

                    if !ignore {
                        // add the configuration to the list
                        result.push(idx.iter().enumerate().map(|(i, pos)| {positions[i][*pos]}).collect());
                    }

                    // update our position vector
                    let mut update_idx = idx.len() - 1;
                    let mut finished = false;
                    loop {
                        idx[update_idx] += 1;
                        if idx[update_idx] >= positions[update_idx].len() {
                            if update_idx == 0 {
                                // we're finished with all permutations
                                finished = true;
                                break;
                            } else {
                                idx[update_idx] = 0;
                                update_idx -= 1;
                            }
                        } else {
                            // finished updating for this permutation
                            break;
                        }
                    }
                    if finished {
                        // finished with everything
                        break;
                    }
                }

                // return result
                Some(result)
            }
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

        let info = LineInfo::from(line.clone());

        match info.query_positions(&query) {
            None => {
                // non-matching line
            },
            Some(positions) => {
                println!("Matching line {:?} with positions {:?}", line, positions);
            }
        }
    }
}
