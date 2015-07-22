use std::io::prelude::*;

use term::terminfo::TermInfo;
use unicode_width::UnicodeWidthStr;

use std::sync::mpsc::{Receiver, Sender};
use std::borrow::Cow;

use std::sync::mpsc;
use std::io;
use std::env;
use std::thread;

use bis_c::{TermTrack, TermSize};
use error::StringError;
use search::SearchBase;

// our user interface instance
pub struct UI {
    // track is a guard, we never touch it manually
    #[allow(dead_code)]
    track: TermTrack,
    size: TermSize,
    query: Sender<String>,
    matches: Receiver<Vec<Cow<'static, str>>>,
    save_cursor: String,
    restore_cursor: String,
    clr_eos: String
}

impl UI {
    pub fn create() -> Result<UI, StringError> {
        debug!("Getting terminfo");
        let mut info = match TermInfo::from_env() {
            Ok(t) => t,
            Err(e) => return Err(StringError::new("Failed to get terminfo",
                                                  Some(Box::new(e))))
        };

        // use terminfo to get control sequences
        debug!("Terminfo: {:?}", info);

        trace!("Getting save_cursor");
        let save_cursor = match info.strings.remove(&format!("sc")) {
            None => return Err(StringError::new("Terminfo did not contain save_cursor", None)),
            Some(item) => match String::from_utf8(item) {
                Err(e) => return Err(StringError::new("save_cursor was not valid utf-8", Some(Box::new(e)))),
                Ok(s) => {
                    trace!("save_cursor: {:?}", s.escape_default());
                    s
                }
            }
        };

        trace!("Getting restore_cursor");
        let restore_cursor = match info.strings.remove(&format!("rc")) {
            None => return Err(StringError::new("Terminfo did not contain restore_cursor", None)),
            Some(item) => match String::from_utf8(item) {
                Err(e) => return Err(StringError::new("restore_cursor was not valid utf-8", Some(Box::new(e)))),
                Ok(s) => {
                    trace!("restore_cursor: {:?}", s.escape_default());
                    s
                }
            }
        };

        trace!("Getting clr_eos");
        let clr_eos = match info.strings.remove(&format!("clr_eos")) {
            None => return Err(StringError::new("Terminfo did not contain clr_eos", None)),
            Some(item) => match String::from_utf8(item) {
                Err(e) => return Err(StringError::new("clr_eos was not valid utf-8", Some(Box::new(e)))),
                Ok(s) => {
                    trace!("restore_cursor: {:?}", s.escape_default());
                    s
                }
            }
        };

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
        thread::spawn(move|| {
            search_thread(query_rx, matches_tx);
        });

        debug!("Creating UI instance");
        let instance = UI {
            track: track,
            size: size,
            query: query_tx,
            matches: matches_rx,
            save_cursor: save_cursor,
            restore_cursor: restore_cursor,
            clr_eos: clr_eos
        };
        
        trace!("Instance creation successful");
        Ok(instance)
    }

    pub fn start(&mut self) -> Result<(), StringError> {
        // assume start on a new line
        // get handles for io
        let stdout = io::stdout();
        let stdin = io::stdin();
        let mut output = stdout.lock();
        let input = stdin.lock();

        let mut query = String::new();

        // draw our prompt and save the cursor
        match write!(output, "Match: {}", self.save_cursor) {
            Err(e) => return Err(StringError::new("Failed to draw prompt", Some(Box::new(e)))),
            Ok(_) => {
                trace!("Drew prompt successfully");
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

        // wait for input
        for maybe_chr in input.chars() {
            let chr = match maybe_chr {
                Err(e) => return Err(StringError::new("Failed to read character", Some(Box::new(e)))),
                Ok(c) => c
            };
            trace!("Got character: {:?}", chr);

            // push the character onto the query string
            query.push(chr);

            // draw the character, save the cursor position, clear the screen after us
            match write!(output, "{}{}{}", chr, self.save_cursor, self.clr_eos) {
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

            // wait for matches
            let matches = match self.matches.recv() {
                Ok(m) => m,
                Err(e) => {
                    return Err(StringError::new("Failed to read matches", Some(Box::new(e))));
                }
            };

            debug!("Got matches: {:?}", &matches);

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
            match write!(output, "{}", self.restore_cursor) {
                Err(e) => return Err(StringError::new("Failed to restore cursor", Some(Box::new(e)))),
                Ok(_) => {
                    trace!("Restored cursor successfully");
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
