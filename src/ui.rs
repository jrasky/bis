use term::terminfo::{TermInfo, Terminal};

use bis_c::TermTrack;
use error::StringError;

// our user interface instance
struct UI {
    track: TermTrack,
    info: TermInfo,
    handle: Box<Terminal>
}
