#BIS#

This is a better search for bash, inspired by flx for emacs. Does fuzzy searching. Probably requires Linux, I haven't tried building or using this on anything else. As with any Rust project, install Rust and Cargo (<http://www.rust-lang.org/>), and then ```cargo run```.

Not documented, not the prettiest code, but it works.

Usage is pretty simple:
 - type characters, bis will try to match them to a line
 - if you see a line you like, press enter, and bis will return and put the line on your prompt (but won't press enter)
 - if you change your mind, press ```C-d``` or ```C-c```. Bis will put what it would have matched to, if anything, but it won't be put on your prompt.
 - if you want to start over, pruss ```C-u``` to clear the line
