use term::terminfo::TermInfo;
use term::StdoutTerminal;

use bis_c::TermTrack;
use error::StringError;

// our user interface instance
struct UI {
    track: TermTrack,
    info: TermInfo,
    handle: Box<StdoutTerminal>
}
