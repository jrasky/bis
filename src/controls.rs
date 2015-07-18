use std::io::ErrorKind::*;
use std::collections::VecMap;
//use unicode::str::*;
use std::io::{Read, Write};

use std::str;
use std::io;
use std::fmt;

use constants::*;
use util::*;
use types::*;
use ioctl::*;

use process::OsPipe;

pub struct Controls {
    stdin: OsPipe,
    stdout: OsPipe,
    stderr: OsPipe,
    cursor: Position,
    tsize: WinSize,
    rows: VecMap<usize>,
    lines: VecMap<usize>,
    line: usize,
    linecol: usize,
    roff: isize
}

impl Controls {
    pub fn new() -> Controls {
        Controls {
            stdin: OsPipe::new(STDIN),
            stdout: OsPipe::new(STDOUT),
            stderr: OsPipe::new(STDERR),
            cursor: Position::new(),
            tsize: WinSize::new(),
            rows: VecMap::new(),
            lines: VecMap::new(),
            line: 0,
            linecol: 0,
            roff: 0
        }
    }

    pub fn update_cursor(&mut self, pos:Position) {
        self.cursor = pos;
        // recalculate rows based on lines
        let old_roff = self.roff;
        let mut line = self.line;
        let mut llen = self.linecol;
        loop {
            let rdiff = llen / self.tsize.col as usize;
            // move up the row difference
            if rdiff > self.cursor.row {
                self.roff -= (rdiff - self.cursor.row) as isize;
                self.cursor.row = 0;
            } else {
                self.cursor.row -= rdiff;
            }
            // move up one line
            if line == 0 {
                break;
            } else {
                line -= 1;
                llen = self.lines[line];
                // move up one row for the line
                if self.cursor.row == 0 {
                    self.roff -= 1;
                } else {
                    self.cursor.row -= 1;
                }
            }
        }
        // the cursor should now be positioned on the correct row
        // for the first line
        // set the cursor to the first column
        self.rows.clear();
        self.cursor.col = 0;
        let lclone = self.lines.clone();
        let lc = self.linecol;
        // grow causes linecol and lines to change, clear these first
        self.lines.clear();
        self.linecol = 0;
        // grow by each line until the end
        for llen in lclone.values() {
            self.grow(*llen);
            // puts an extra new_row at the end of the last line,
            // but that doesn't really matter too much
            self.new_row();
        }
        // we are now at the end, restore the original position and lines
        self.cursor = pos;
        self.roff = old_roff;
        self.lines = lclone;
        self.linecol = lc;
        // and done, we should now have a correct row representation
        // based on the lines previously typed, and we should be
        // correctly positioned within these rows.
    }
    
    pub fn update_size(&mut self, size:WinSize) {
        self.tsize = size;
    }

    pub fn clear_rows(&mut self) {
        self.rows.clear();
        self.roff = 0;
        self.linecol = 0;
        self.line = 0;
        self.lines.clear();
    }

    pub fn get_row(&mut self) -> usize {
        self.cursor.row
    }

    pub fn width(&mut self) -> usize {
        self.tsize.col as usize
    }

    pub fn get_pos(&mut self) -> Position {
        self.cursor
    }

    pub fn row_length(&mut self) -> usize {
        if !self.rows.contains_key(&((self.cursor.row as isize + self.roff) as usize)) {
            return 1;
        } else {
            return self.rows[(self.cursor.row as isize + self.roff) as usize];
        }
    }

    pub fn save_row(&mut self, len:usize) {
        if !self.rows.contains_key(&((self.cursor.row as isize + self.roff) as usize)) {
            self.rows.insert((self.cursor.row as isize + self.roff) as usize, len);
        } else {
            self.rows[(self.cursor.row as isize + self.roff) as usize] = len;
        }
    }

    pub fn update_line(&mut self, len:usize) {
        self.linecol = len;
        if self.lines.contains_key(&self.line) {
            self.lines[self.line] = self.linecol;
        } else {
            self.lines.insert(self.line, self.linecol);
        }
    }

    fn line_length(&mut self) -> usize {
        if self.lines.contains_key(&self.line) {
            self.lines[self.line]
        } else {
            0
        }
    }

    fn move_right(&mut self, by:usize) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        self.cursor.col += by;
        self.linecol += by;
        let mut row_size = self.row_length();
        if self.cursor.col > row_size {
            loop {
                if self.cursor.col > row_size {
                    self.cursor.col -= row_size;
                    self.cursor.row += 1;
                    row_size = self.row_length();
                } else {
                    break;
                }
            }
            if self.cursor.row > self.tsize.row as usize {
                self.roff += self.cursor.row as isize - self.tsize.row as isize;
                self.cursor.row = self.tsize.row as usize;
            }
        }
        let llen = self.line_length();
        if self.linecol > llen {
            self.linecol = 0;
            self.line += 1;
        }
    }

    fn move_left(&mut self, mut by:usize) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        let mut lby = by;
        loop {
            if lby > self.linecol {
                if self.line > 0 {
                    self.line -= 1;
                }
                let llen = self.line_length();
                lby -= self.linecol + 1;
                self.linecol = llen;
            } else {
                self.linecol -= lby;
                break;
            }
        }
        loop {
            if by >= self.cursor.col {
                by -= self.cursor.col;
                if self.cursor.row == 0 {
                    self.roff -= 1;
                } else {
                    self.cursor.row -= 1;
                }
                self.cursor.col = self.row_length();
            } else {
                self.cursor.col -= by;
                break;
            }
        }
    }

    pub fn grow_check(&mut self, len:usize) -> bool {
        self.cursor.col + len > self.tsize.col as usize
    }

    pub fn grow(&mut self, by:usize) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        self.cursor.col += by;
        self.linecol += by;
        if self.lines.contains_key(&self.line) {
            self.lines[self.line] += by;
        } else {
            self.lines.insert(self.line, self.linecol);
        }
        if self.cursor.col > self.tsize.col as usize {
            let col = self.tsize.col as usize;
            self.save_row(col);
            self.cursor.row += self.cursor.col / self.tsize.col as usize;
            self.cursor.col = self.cursor.col % self.tsize.col as usize;
            if self.cursor.row > self.tsize.row as usize {
                self.roff += self.cursor.row as isize - self.tsize.row as isize;
                self.cursor.row = self.tsize.row as usize;
            }
        }
        let col = self.cursor.col;
        self.save_row(col);
    }

    pub fn shrink(&mut self, mut by:usize) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        let mut lby = by;
        loop {
            if lby > self.linecol {
                if self.line == 0 {
                    self.linecol = 0;
                    self.lines[0] = 0;
                    break;
                } else {
                    self.lines[self.line] -= self.linecol;
                    lby -= self.linecol + 1;
                    if self.line > 0 {
                        self.line -= 1;
                    }
                    self.linecol = self.line_length();
                }
            } else {
                self.linecol -= lby;
                self.lines[self.line] -= lby;
                break;
            }
        }
        loop {
            if by >= self.cursor.col {
                by -= self.cursor.col;
                self.rows.remove(&((self.cursor.row as isize + self.roff) as usize));
                if self.cursor.row == 0 {
                    self.roff -= 1;
                } else {
                    self.cursor.row -= 1;
                }
                self.cursor.col = self.row_length();
            } else {
                self.cursor.col -= by;
                break;
            }
        }
        let col = self.cursor.col;
        self.save_row(col);
    }

    fn new_row(&mut self) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        if self.cursor.row == self.tsize.row as usize {
            self.roff += 1;
        } else {
            self.cursor.row += 1;
        }
        self.cursor.col = 1;
    }

    fn new_line(&mut self) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        self.new_row();
        self.line += 1;
        self.linecol = 0;
        if self.lines.contains_key(&self.line) {
            self.lines[self.line] = self.linecol;
        } else {
            self.lines.insert(self.line, self.linecol);
        }
    }

    /* out* and err* functions are assumed to be output
       functions */

    pub fn outc(&mut self, ch:char) {
        self.stdout.write_char(ch).unwrap();
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        if ch == NL {
            self.new_line();
        } else {
            if self.grow_check(1) {
                self.stdout.write_char(SPC).unwrap();
                self.stdout.write_char(DEL).unwrap();
            }
            self.grow(1);
        }
    }

    pub fn outs(&mut self, s:&str) {
        if self.tsize.col == 0 || self.tsize.row == 0 {
            self.stdout.write_str(s).unwrap();
            return;
        }
        let mut splits = NL_REGEX.split(s);
        match splits.next() {
            Some(part) => {
                self.stdout.write_str(part).unwrap();
                if self.cursor.col + part.len() - 1 == self.tsize.col as usize {
                    self.stdout.write_char(SPC).unwrap();
                    self.stdout.write_char(DEL).unwrap();
                }
                self.grow(part.len());
                for part in splits {
                    self.new_line();
                    self.stdout.write_char(NL).unwrap();
                    self.stdout.write_str(part).unwrap();
                    if self.cursor.col + part.len() - 1 == self.tsize.col as usize {
                        self.stdout.write_char(SPC).unwrap();
                        self.stdout.write_char(DEL).unwrap();
                    }
                    self.grow(part.len());
                }
            },
            _ => {}
        }
    }

    pub fn outf(&mut self, args:fmt::Arguments) {
        self.outs(fmt::format(args).as_slice());
    }

    pub fn err(&mut self, s:&str) {
        if self.tsize.col == 0 || self.tsize.row == 0 {
            self.stderr.write_str(s).unwrap();
            return;
        }
        self.stderr.write_str(s).unwrap();
        let mut splits = NL_REGEX.split(s);
        match splits.next() {
            Some(part) => {
                self.grow(part.len());
                for part in splits {
                    self.new_row();
                    if self.cursor.col + part.len() == self.tsize.col as usize {
                        self.stdout.write_char(NL).unwrap();
                    }
                    self.grow(part.len());
                }
            },
            _ => {}
        }
    }

    pub fn errf(&mut self, args:fmt::Arguments) {
        self.err(fmt::format(args).as_slice());
    }

    pub fn del(&mut self) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        if self.cursor.col > 1 {
            self.stdout.write_char(DEL).unwrap();
            self.shrink(1);
        } else {
            self.shrink(1);
            self.move_to_pointer();
        }
    }

    pub fn read(&mut self) -> io::Result<char> {
        // Below lifted almost verbatim from rust's read_char.
        // In compliance with MIT, nothing to worry about
        let first_byte = {
            let mut buf = [0; 1];
            try!(self.stdin.read(&mut buf));
            buf[0]
        };
        let width = utf8_char_width(first_byte);
        if width == 1 { return Ok(first_byte as char) }
        if width == 0 { return Err(io::Error::new(InvalidInput, "invalid input",
                                                  Some(format!("Character was not utf-8")))) }
        let mut buf = [first_byte, 0, 0, 0];
        {
            let mut start = 1;
            while start < width {
                match try!(self.stdin.read(&mut buf[start..width])) {
                    n if n == width - start => break,
                    n if n < width - start => { start += n; }
                    _ => return Err(io::Error::new(InvalidInput, "invalid input",
                                                   Some(format!("Character was not utf-8")))),
                }
            }
        }
        match str::from_utf8(&buf[..width]).ok() {
            Some(s) => Ok(s.char_at(0)),
            None => Err(io::Error::new(InvalidInput, "invalid input",
                                       Some(format!("Character was not utf-8"))))
        }
    }
    
    pub fn cursor_left(&mut self) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        if self.cursor.col > 1 {
            self.stdout.write_char(DEL).unwrap();
            self.move_left(1);
        } else {
            self.move_left(1);
            self.move_to_pointer();
        }
    }

    pub fn cursor_right(&mut self) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        if self.cursor.col < self.row_length() {
            self.stdout.write_str(CRSR_RIGHT).unwrap();
            self.move_right(1);
        } else {
            self.move_right(1);
            self.move_to_pointer();
        }
    }
    
    pub fn cursors_left(&mut self, by:usize) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        if by == 0 {
            return;
        } else if by <= 3 && self.cursor.col > by {
            self.stdout.write_str(build_string(DEL, by).as_slice()).unwrap();
            self.move_left(by);
        } else if self.cursor.col > by {
            self.stdout.write_fmt(format_args!("{}{}D", ANSI_BEGIN, by)).unwrap();
            self.move_left(by);
        } else {
            self.move_left(by);
            self.move_to_pointer();
        }
    }

    pub fn cursors_right(&mut self, by:usize) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        if by == 0 {
            return;
        } else if by + self.cursor.col <= self.row_length() {
            self.stdout.write_fmt(format_args!("{}{}C", ANSI_BEGIN, by)).unwrap();
            self.move_right(by);
        } else {
            self.move_right(by);
            self.move_to_pointer();
        }
    }

    pub fn clear_line(&mut self) {
        self.stdout.write_str(ANSI_BEGIN).unwrap();
        self.stdout.write_char('K').unwrap();
        let col = self.cursor.col;
        self.save_row(col);
    }

    pub fn clear_line_to(&mut self, len:usize) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        let old = self.cursor;
        let mut total = self.cursor.col + len;
        let mut rlen = self.row_length();
        loop {
            self.clear_line();
            if total > rlen {
                total -= rlen;
                self.next_start();
                rlen = self.row_length();
            } else {
                break;
            }
        }
        self.move_to(old);
    }

    pub fn next_start(&mut self) {
        if self.tsize.col == 0 || self.tsize.row == 0 {return}
        self.new_row();
        let row = self.cursor.row;
        self.move_to(Position {
            col: 1,
            row: row
        });
    }

    pub fn query_cursor(&mut self) {
        self.stdout.write_str(CRSR_POS).unwrap();
    }

    fn move_to_pointer(&mut self) {
        let (row, col) = (self.cursor.row, self.cursor.col);
        self.stdout.write_fmt(format_args!("{}{};{}f", ANSI_BEGIN, row, col)).unwrap();
    }

    pub fn move_to(&mut self, pos:Position) {
        self.cursor = pos;
        self.move_to_pointer();
    }

    pub fn bell(&mut self) {
        self.stdout.write_char(BEL).unwrap();
    }
}

