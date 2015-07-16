#![feature(iter_arith)]
use std::fs::File;
use std::io::{BufReader, BufRead, Write};
use std::collections::{HashMap};

use std::env;
use std::io;

const WHITESPACE_FACTOR: isize = 5;
const WHITESPACE_REDUCE: isize = 2;
const CLASS_FACTOR: isize = 3;
const FIRST_FACTOR: isize = 3;
const CLASS_REDUCE: isize = 2;

const DIST_WEIGHT: isize = -10;
const HEAT_WEIGHT: isize = 5;
const LINE_REDUCE: isize = 50;

const MAX_LEN: usize = 80;

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
    First,
    Other
}

impl<T: Into<String>> From<T> for LineInfo {
    fn from(item: T) -> LineInfo {
        let mut map: HashMap<char, Vec<usize>> = HashMap::new();
        let mut heat = vec![];
        let line = item.into();

        let mut ws_score = 0;
        let mut cs_score = 0;
        let mut cur_class = CharClass::First;
        // character class changes don't stack
        let mut cs_change = false;

        for (idx, c) in line.chars().enumerate() {
            // don't map whitespace
            if !c.is_whitespace() {
                // update the character class change score if needed
                if cur_class == CharClass::First {
                    // add the first character factor on top of class change
                    cs_score += FIRST_FACTOR;
                }
                if c.is_numeric() {
                    if cur_class != CharClass::Numeric {
                        cur_class = CharClass::Numeric;
                        if !cs_change {
                            cs_score += CLASS_FACTOR;
                            cs_change = true;
                        }
                    } else {
                        cs_change = false;
                    }
                } else if c.is_alphabetic() {
                    if cur_class != CharClass::Alphabetic {
                        cur_class = CharClass::Alphabetic;
                        if !cs_change {
                            cs_score += CLASS_FACTOR;
                            cs_change = true;
                        }
                    } else {
                        cs_change = false;
                    }
                } else {
                    if cur_class != CharClass::Other {
                        cur_class = CharClass::Other;
                        if !cs_change {
                            cs_score += CLASS_FACTOR;
                            cs_change = true;
                        }
                    } else {
                        cs_change = false;
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
            if !cs_change {
                cs_score /= CLASS_REDUCE;
            }
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

    fn query_score<T: AsRef<str>>(&self, query: T) -> Option<isize> {
        match self.query_positions(&query) {
            None => None,
            Some(positions) => {
                let mut top_score = None;
                for pgroup in positions.iter() {
                    // find the average distance between the indexes
                    let mut dist_total = 0;
                    let mut dist_count = 0;
                    for i in 0..pgroup.len() - 1 {
                        dist_total += (pgroup[i + 1] - pgroup[i]) as isize;
                        dist_count += 1;
                    }
                    // sum the heatmap
                    let heat_sum: isize = pgroup.iter().map(|pos| {self.heatmap[*pos]}).sum();
                    let score = (dist_total / dist_count) * DIST_WEIGHT +
                        heat_sum * HEAT_WEIGHT;
                    match top_score {
                        None => top_score = Some(score),
                        Some(last) => {
                            if score > last {
                                top_score = Some(score);
                            }
                        }
                    }
                }

                // return the result
                top_score
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

    let mut line_number = -1;
    let mut best_match = None;
    let mut best_score = None;

    for m_line in input_file.lines() {
        let line = match m_line {
            Ok(l) => l,
            Err(e) => panic!("Failed to read line: {}", e)
        };

        if line.len() > MAX_LEN {
            // ignore this line
            continue;
        }

        let info = LineInfo::from(line.clone());

        line_number += 1;

        let line_score = match info.query_score(&query) {
            None => {
                // non-matching line
                continue;
            },
            Some(score) => {
                score + line_number / LINE_REDUCE
            }
        };

        println!("Line {:?} score {}", &line, line_score);

        match best_score {
            None => {
                best_score = Some(line_score);
                best_match = Some(line);
            },
            Some(last) => {
                if line_score >= last {
                    best_score = Some(line_score);
                    best_match = Some(line);
                }
            }
        }
    }

    match best_match {
        Some(line) => {
            println!("Best match: {:?}", line);
        },
        None => {
            println!("No best match found");
        }
    }
}
