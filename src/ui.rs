use std::io::prelude::*;

use term::terminfo::TermInfo;
use unicode_width::UnicodeWidthStr;

use std::sync::mpsc::{Receiver, Sender};
use std::borrow::{Cow, Borrow};
use std::collections::HashMap;
use std::ffi::OsString;
use std::iter::FromIterator;

use std::sync::mpsc;
use std::io;
use std::env;
use std::thread;

use bis_c::{TermTrack, TermSize};
use error::StringError;
use search::SearchBase;

// TermControl contains utility funcitons for terminfo
#[derive(Debug)]
struct TermControl {
    strings: HashMap<String, String>
}

#[derive(PartialEq, Clone, Debug)]
enum TermStack {
    // here for correctness
    #[allow(dead_code)]
    Str(String),
    Int(isize),
    // here for correctness
    #[allow(dead_code)]
    Bool(bool)
}

// our user interface instance
pub struct UI {
    // track is a guard, we never touch it manually
    #[allow(dead_code)]
    track: TermTrack,
    size: TermSize,
    control: TermControl,
    query: Sender<String>,
    matches: Receiver<Vec<Cow<'static, str>>>,
    chars: Receiver<char>
}

impl Drop for UI {
    fn drop(&mut self) {
        debug!("Clearing screen on exit");
        let mut output = io::stdout();
        match write!(output, "{}", self.control.get_string("clr_eos".to_owned(), vec![]).unwrap_or(format!(""))) {
            Ok(_) => {
                trace!("Cleared screen successfully");
            },
            Err(e) => {
                error!("Failed to clear screen: {}", e);
            }
        }
        debug!("Flushing output");
        match output.flush() {
            Ok(_) => {
                trace!("Flushed screen successfully");
            },
            Err(e) => {
                error!("Failed to flush screen: {}", e);
            }
        }
    }
}

impl TermControl {
    pub fn create() -> Result<TermControl, StringError> {
        debug!("Getting terminal info");
        let info = match TermInfo::from_env() {
            Ok(info) => info,
            Err(e) => return Err(StringError::new("Failed to get TermInfo", Some(Box::new(e))))
        };

        trace!("Got terminfo: {:?}", info);

        let mut strings = HashMap::default();

        for (name, value) in info.strings.into_iter() {
            strings.insert(name, match String::from_utf8(value) {
                Ok(s) => s,
                Err(e) => return Err(StringError::new("Failed to convert value into an OsString", Some(Box::new(e))))
            });
        }

        // right now all we care about are the strings
        Ok(TermControl {
            strings: strings
        })
    }

    pub fn get_string<T: Borrow<String>>(&mut self, name: T, params: Vec<TermStack>) -> Option<String> {
        // only implement what we're actually using in the UI
        let sequence = match self.strings.get(name.borrow()) {
            None => {
                trace!("No match for string: {:?}", name.borrow());
                return None;
            },
            Some(s) => {
                trace!("Matched string: {:?}", s);
                s.clone()
            }
        };

        let mut escaped = false;
        let mut stack: Vec<TermStack> = vec![];
        let mut result = String::default();
        let mut escape = String::default();

        // only implement the sequences we care about

        for c in sequence.chars() {
            if !escaped {
                if c == '%' {
                    escaped = true;
                } else {
                    result.push(c);
                }
            } else if escape.is_empty() {
                if c == 'd' {
                    match stack.pop() {
                        Some(TermStack::Int(c)) => {
                            result.push_str(format!("{}", c).as_ref());
                        },
                        Some(o) => {
                            error!("Numeric print on non-numeric type: {:?}", o);
                        },
                        None => {
                            error!("Stack was empty on print");
                        }
                    }

                    escaped = false;
                } else if c == 'p' {
                    escape.push('p');
                } else {
                    error!("Unknown escape character: {:?}", c);
                    escaped = false;
                }
            } else {
                if escape == "p" {
                    match c.to_digit(10) {
                        Some(idx) => {
                            if idx != 0 {
                                match params.get(idx as usize - 1) {
                                    Some(item) => {
                                        stack.push(item.clone())
                                    },
                                    None => {
                                        error!("There was no parameter {}", idx);
                                    }
                                }
                            } else {
                                error!("Tried to print 0th paramater");
                            }
                        },
                        None => {
                            error!("Paramater number was not a digit");
                        }
                    }

                    escape.clear();
                    escaped = false;
                } else {
                    error!("Unknown escape sequence: {:?}", escape);
                    escape.clear();
                    escaped = false;
                }
            }
        }

        trace!("Returning result: {:?}", result);

        // return result
        Some(result)
    }
}

impl UI {
    pub fn create() -> Result<UI, StringError> {
        debug!("Creating TermControl");
        let control = try!(TermControl::create());

        trace!("Got TermControl: {:?}", control);

        let mut track = TermTrack::default();

        debug!("Getting terminal size");
        let size = match track.get_size() {
            Err(e) => return Err(StringError::new("Failed to get terminal size", Some(Box::new(e)))),
            Ok(s) => {
                trace!("Terminal size: {:?}", s);
                s
            }
        };

        debug!("Preparing terminal");
        match track.prepare() {
            Err(e) => return Err(StringError::new("Failed to prepare terminal", Some(Box::new(e)))),
            Ok(_) => {
                trace!("Terminal prepared successfully");
            }
        }

        debug!("Starting search thread");

        trace!("Creating thread primitives");
        let (query_tx, query_rx) = mpsc::channel();
        let (matches_tx, matches_rx) = mpsc::channel();

        trace!("Starting thread");
        thread::spawn(move || {
            search_thread(query_rx, matches_tx);
        });

        debug!("Starting input thread");

        trace!("Creating thread primitives");
        let (chars_tx, chars_rx) = mpsc::channel();

        trace!("Starting thread");
        thread::spawn(move || {
            input_thread(chars_tx);
        });

        debug!("Creating UI instance");
        let instance = UI {
            track: track,
            size: size,
            control: control,
            query: query_tx,
            matches: matches_rx,
            chars: chars_rx
        };
        
        trace!("Instance creation successful");
        Ok(instance)
    }

    pub fn start(&mut self) -> Result<(), StringError> {
        // assume start on a new line
        // get handles for io
        let stdout = io::stdout();
        let mut output = stdout.lock();

        let mut query = String::new();

        // make space for our matches
        match write!(output, "{}{}", String::from_iter(vec!['\n'; 10].into_iter()),
                     self.control.get_string("cuu".to_owned(), vec![TermStack::Int(10)]).unwrap_or(format!(""))) {
            Err(e) => return Err(StringError::new("Failed to create space", Some(Box::new(e)))),
            Ok(_) => {
                trace!("Successfully created space on terminal");
            }
        }

        // draw our prompt and save the cursor
        debug!("Drawing prompt");
        match write!(output, "Match: {}",
                     self.control.get_string("sc".to_owned(), vec![]).unwrap_or(format!(""))) {
            Err(e) => return Err(StringError::new("Failed to draw prompt", Some(Box::new(e)))),
            Ok(_) => {
                trace!("Drew prompt successfully");
            }
        }

        // flush the output
        debug!("Flushing output");
        match output.flush() {
            Ok(_) => {
                trace!("Successfully flushed output");
            },
            Err(e) => {
                return Err(StringError::new("Failed to flush output", Some(Box::new(e))));
            }
        }

        // are you kidding me with this stupid macro bullshit
        let matches_chan = &self.matches;
        let chars_chan = &self.chars;

        loop {
            select! {
                maybe_matches = matches_chan.recv() => {
                    let matches = match maybe_matches {
                        Ok(m) => m,
                        Err(e) => return Err(StringError::new("Query thread hung up", Some(Box::new(e))))
                    };
                    debug!("Got matches: {:?}", matches);

                    // draw the matches
                    for item in matches.into_iter() {
                        if UnicodeWidthStr::width(item.as_ref()) > self.size.cols {
                            let mut owned = item.into_owned();
                            while UnicodeWidthStr::width((&owned as &AsRef<str>).as_ref()) > self.size.cols {
                                // truncate long lines
                                owned.pop();
                            }
                            // draw the truncated item
                            match write!(output, "\n{}", owned) {
                                Err(e) => return Err(StringError::new("Failed to draw match", Some(Box::new(e)))),
                                Ok(_) => {
                                    trace!("Drew match successfully");
                                }
                            }
                        } else {
                            // draw the match after a newline
                            match write!(output, "\n{}", item) {
                                Err(e) => return Err(StringError::new("Failed to draw match", Some(Box::new(e)))),
                                Ok(_) => {
                                    trace!("Drew match successfully");
                                }
                            }
                        }
                    }

                    // restore the cursor
                    match write!(output, "{}", self.control.get_string("rc".to_owned(), vec![]).unwrap_or(format!(""))) {
                        Err(e) => return Err(StringError::new("Failed to restore cursor", Some(Box::new(e)))),
                        Ok(_) => {
                            trace!("Restored cursor successfully");
                        }
                    }
                },
                maybe_chr = chars_chan.recv() => {
                    let chr = match maybe_chr {
                        Ok(c) => c,
                        Err(e) => {
                            // io hung up, exit
                            debug!("IO thread hung up: {:?}", e);
                            break;
                        }
                    };
                    debug!("Got character: {:?}", chr);

                    // push the character onto the query string
                    query.push(chr);

                    // draw the character, save the cursor position, clear the screen after us
                    match write!(output, "{}{}{}", chr,
                                 self.control.get_string("sc".to_owned(), vec![]).unwrap_or(format!("")),
                                 self.control.get_string("clr_eos".to_owned(), vec![]).unwrap_or(format!(""))) {
                        Err(e) => return Err(StringError::new("Failed to output character", Some(Box::new(e)))),
                        Ok(_) => {
                            trace!("Outputted character successfully");
                        }
                    }

                    // send the search thread our query
                    debug!("Sending {} to search thread", &query);
                    match self.query.send(query.clone()) {
                        Ok(_) => {
                            trace!("Send successful");
                        },
                        Err(e) => {
                            return Err(StringError::new("Failed to send to search thread", Some(Box::new(e))));
                        }
                    }
                }
            }

            // flush the output
            match output.flush() {
                Ok(_) => {
                    trace!("Successfully flushed output");
                },
                Err(e) => {
                    return Err(StringError::new("Failed to flush output", Some(Box::new(e))));
                }
            }
        }

        // Return success
        Ok(())
    }
}

// this thread waits for queries, and responds with search matches
pub fn search_thread(query: Receiver<String>,
                     matches: Sender<Vec<Cow<'static, str>>>) {
    debug!("Starting query thread");

    debug!("Getting history path");
    let history_path = match env::var("HISTFILE") {
        Ok(p) => {
            trace!("Got history path: {:?}", p);
            p
        },
        Err(e) => panic!("Failed to get bash history file: {}", e)
    };

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

    debug!("Starting query loop");

    loop {
        trace!("Waiting for a query");
        match query.recv() {
            Err(e) => {
                debug!("Search thread exiting: {}", e);
                break;
            },
            Ok(q) => {
                debug!("Got query: {:?}", q);
                let result = base.query(q);
                debug!("Got result: {:?}", result);
                match matches.send(result) {
                    Err(e) => {
                        debug!("Search thread exiting: {}", e);
                        break;
                    },
                    Ok(_) => {
                        trace!("Matches sent successfully");
                    }
                }
            }
        }
    }
}

// this thread waits for input on stdin and sends that input back
fn input_thread(chars: Sender<char>) {
    debug!("Starting input thread");

    debug!("Getting stdin lock");
    let input = io::stdin();

    for maybe_chr in input.chars() {
        match maybe_chr {
            Err(e) => {
                debug!("Input thread exiting: {}", e);
                break;
            },
            Ok(c) => {
                debug!("Got character: {:?}", c);
                match chars.send(c) {
                    Err(e) => {
                        debug!("Search thread exiting: {:?}", e);
                    },
                    Ok(_) => {
                        trace!("Character sent successfully");
                    }
                }
            }
        }
    }

    debug!("Input thread ran out of input");
}
