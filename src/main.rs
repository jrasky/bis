#![feature(collections)]
#![feature(into_cow)]
#![feature(iter_arith)]
use std::fs::File;
use std::io::{BufReader, BufRead, Write};
use std::collections::{HashMap, BinaryHeap};
use std::borrow::{Cow, IntoCow, Borrow};
use std::error::Error;

use std::env;
use std::io;
use std::cmp;
use std::fmt;
use std::path;

const WHITESPACE_FACTOR: isize = 5;
const WHITESPACE_REDUCE: isize = 2;
const CLASS_FACTOR: isize = 3;
const FIRST_FACTOR: isize = 3;
const CLASS_REDUCE: isize = 2;

const DIST_WEIGHT: isize = -10;
const HEAT_WEIGHT: isize = 5;
const FACTOR_REDUCE: isize = 50;

const MAX_LEN: usize = 80;

const MATCH_NUMBER: usize = 10;

#[derive(Debug)]
struct LineInfo {
    char_map: HashMap<char, Vec<usize>>,
    heatmap: Vec<isize>,
    pub factor: isize
}

#[derive(Debug)]
struct LineMatch {
    score: isize,
    factor: isize,
    line: Cow<'static, str>
}

#[derive(Debug)]
struct SearchBase {
    lines: HashMap<Cow<'static, str>, LineInfo>
}

#[derive(Debug)]
struct StringError {
    description: String,
    cause: Option<Box<Error>>
}

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for StringError {
    fn description(&self) -> &str {
        self.description.as_ref()
    }

    fn cause(&self) -> Option<&Error> {
        match self.cause {
            None => None,
            Some(ref error) => Some(error.borrow())
        }
    }
}

impl StringError {
    fn new<T: Into<String>>(description: T, cause: Option<Box<Error>>) -> StringError {
        StringError {
            description: description.into(),
            cause: cause
        }
    }
}

impl Default for SearchBase {
    fn default() -> SearchBase {
        SearchBase {
            lines: HashMap::default()
        }
    }
}

impl SearchBase {
    pub fn read_history<T: AsRef<path::Path>>(&mut self, path: T) -> Result<isize, StringError> {
        let input_file = match File::open(path) {
            Ok(f) => BufReader::new(f),
            Err(e) => return Err(StringError::new("Could not open history file", Some(Box::new(e))))
        };

        let mut line_number = -1;

        for m_line in input_file.lines() {
            let line = match m_line {
                Ok(mut line) => {
                    if line.len() > MAX_LEN {
                        let mut cut_at = None;
                        for (idx, _) in line.char_indices() {
                            if idx > MAX_LEN {
                                cut_at = Some(idx);
                                break;
                            }
                        }
                        match cut_at {
                            None => {
                                // do nothing, last character spans the 80th byte
                            },
                            Some(idx) => {
                                line.truncate(idx);
                            }
                        }
                    }
                    // return the result
                    line
                },
                Err(e) => {
                    return Err(StringError::new("Failed to read line", Some(Box::new(e))));
                }
            };

            line_number += 1;

            // generate the line info
            let info = LineInfo::new(&line, line_number);

            // insert the line into the map
            self.lines.insert(line.into_cow(), info);
        }

        Ok(line_number)
    }

    pub fn query_inplace<T: AsRef<str>>(&self, query: T, matches: &mut BinaryHeap<LineMatch>) {
        // search for a match
        for (line, info) in self.lines.iter() {
            let line_score = match info.query_score(&query) {
                None => {
                    // non-matching line
                    continue;
                },
                Some(score) => {
                    score
                }
            };

            // negate everything so we can use push_pop
            let match_item = LineMatch {
                score: -line_score,
                factor: -info.factor,
                line: line.clone()
            };
            let matches_len = matches.len();
            let matches_capacity = matches.capacity();
            let insert;
            match matches.peek() {
                None => {
                    insert = true;
                },
                Some(item) => {
                    if &match_item < item || matches_len < matches_capacity {
                        insert = true
                    } else {
                        insert = false;
                    }
                }
            }
            if insert {
                if matches_len < matches_capacity {
                    matches.push(match_item);
                } else {
                    matches.push_pop(match_item);
                }
            }
        }
    }

    pub fn query<T: AsRef<str>>(&self, query: T) -> Vec<Cow<str>> {
        // allocate the match object
        let mut matches: BinaryHeap<LineMatch> = BinaryHeap::with_capacity(MATCH_NUMBER);

        self.query_inplace(query, &mut matches);

        // result contains a vector of the top MATCH_NUMBER lines, in descending score order
        matches.into_sorted_vec().into_iter().map(|x| {x.line}).collect()
    }
}


#[derive(PartialEq)]
enum CharClass {
    Whitespace,
    Numeric,
    Alphabetic,
    First,
    Other
}

impl Ord for LineMatch {
    fn cmp(&self, other: &LineMatch) -> cmp::Ordering {
        match self.score.cmp(&other.score) {
            cmp::Ordering::Equal => self.factor.cmp(&other.factor),
            order => order
        }
    }
}

impl PartialOrd for LineMatch {
    fn partial_cmp(&self, other: &LineMatch) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for LineMatch {
    fn eq(&self, other: &LineMatch) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl Eq for LineMatch {}

impl LineInfo {
    fn new<T: AsRef<str>>(item: T, factor: isize) -> LineInfo {
        let mut map: HashMap<char, Vec<usize>> = HashMap::new();
        let mut heat = vec![];
        let line = item.as_ref();

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
            char_map: map,
            heatmap: heat,
            factor: factor
        }
    }

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
        match self.query_positions(query) {
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
                match top_score {
                    None => None,
                    Some(score) => {
                        Some(score + self.factor / FACTOR_REDUCE)
                    }
                }
            }
        }
    }
}

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
