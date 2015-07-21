use std::io::prelude::*;

use term::terminfo::TermInfo;

use std::io::Stdout;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Barrier};
use std::borrow::Cow;

use std::sync::mpsc;
use std::io;
use std::env;
use std::thread;
use std::mem;

use bis_c::{TermTrack, TermSize};
use error::StringError;
use search::SearchBase;

// our user interface instance
struct UI {
    track: TermTrack,
    size: TermSize,
    handle: Stdout,
    query: Sender<Cow<'static, str>>,
    matches: Receiver<Vec<Cow<'static, str>>>,
    save_cursor: String,
    restore_cursor: String,
    clr_eos: String
}

impl UI {
    pub fn create() -> Result<UI, StringError> {
        debug!("Getting terminfo");
        let info = match TermInfo::from_env() {
            Ok(t) => t,
            Err(e) => return Err(StringError::new("Failed to get terminfo",
                                                  Some(Box::new(e))))
        };

        // use terminfo to get control sequences
        debug!("Checking terminal capabilities");

        trace!("Getting save_cursor");
        let save_cursor = match info.strings.get(&format!("sc")) {
            None => return Err(StringError::new("Terminfo did not contain save_cursor", None)),
            Some(item) => match String::from_utf8(item) {
                Err(e) => return Err(StringError::new("save_cursor was not valid utf-8", Some(Box::new(e)))),
                Some(s) => {
                    trace!("save_cursor: {:?}", s.escape_default());
                    s
                }
            }
        };

        trace!("Getting restore_cursor");
        let save_cursor = match info.strings.get(&format!("rc")) {
            None => return Err(StringError::new("Terminfo did not contain restore_cursor", None)),
            Some(item) => match String::from_utf8(item) {
                Err(e) => return Err(StringError::new("restore_cursor was not valid utf-8", Some(Box::new(e)))),
                Some(s) => {
                    trace!("restore_cursor: {:?}", s.escape_default());
                    s
                }
            }
        };

        trace!("Getting clr_eos");
        let clr_eos = match info.strings.get(&format!("ed")) {
            None => return Err(StringError::new("Terminfo did not contain clr_eos", None)),
            Some(item) => match String::from_utf8(item) {
                Err(e) => return Err(StringError::new("clr_eos was not valid utf-8", Some(Box::new(e)))),
                Some(s) => {
                    trace!("restore_cursor: {:?}", s.escape_default());
                    s
                }
            }
        };

        let mut track = TermTrack::default();

        debug!("Getting terminal size");
        match track.get_size() {
            Err(e) => return Err(StringError::new("Failed to get terminal size", Some(Box::new(e)))),
            Some(s) => {
                trace!("Terminal size: {:?}", size);
                s
            }
        }

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
        let mut instance = UI {
            track: track,
            handle: io::stdout(),
            query: query_tx,
            matches: matches_rx,
            save_cursor: save_cursor,
            restore_cursor: restore_cursor,
            clr_eos: clr_eos
        };
        
        trace!("Instance creation successful");
        Ok(instance)
    }
}

pub fn search_thread(query: Receiver<Cow<'static, str>>,
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
                debug!("Search thread exiting");
                break;
            },
            Ok(q) => {
                debug!("Got query: {:?}", q);
                let result = base.query(q);
                debug!("Got result: {:?}", result);
                matches.send(result);
            }
        }
    }
}
