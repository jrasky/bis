// bindings into bis_c.c

use std::ffi::CString;

use error::StringError;

// this object exists to track Rust's memory model
// that way the terminal is restored when the main
// thread exits
#[derive(Debug)]
pub struct TermTrack;

#[derive(Debug, Clone, Copy)]
pub struct TermSize {
    pub rows: usize,
    pub cols: usize
}

mod c {
    use libc::*;

    use std::ffi;
    use std::io;

    use error::StringError;

    #[repr(C)]
    struct bis_error_info_t {
        error_str: *const c_char,
        is_errno: c_char
    }

    #[repr(C)]
    pub struct bis_term_size_t {
        pub rows: c_ushort,
        pub cols: c_ushort
    }

    extern "C" {
        static mut bis_error_info: bis_error_info_t;
        
        pub fn bis_prepare_terminal() -> c_int;
        pub fn bis_restore_terminal() -> c_int;
        pub fn bis_get_terminal_size(size: *mut bis_term_size_t) -> c_int;
        pub fn bis_mask_sigint() -> c_int;
        pub fn bis_wait_sigint() -> c_int;
        pub fn bis_insert_input(input: *const c_char) -> c_int;
    }

    pub unsafe fn get_bis_error() -> StringError {
        let error_cstr = ffi::CStr::from_ptr(bis_error_info.error_str);
        StringError::new(error_cstr.to_string_lossy().into_owned(),
                         match bis_error_info.is_errno {
                             1 => Some(Box::new(io::Error::last_os_error())),
                             _ => None
                         })
    }
}

impl Default for TermTrack {
    fn default() -> TermTrack {
        TermTrack
    }
}

impl Drop for TermTrack {
    fn drop(&mut self) {
        match self.restore() {
            Ok(()) => {
                trace!("Successfully restored terminal");
            },
            Err(e) => {
                error!("Error restoring terminal: {}", e);
            }
        }
    }
}

impl TermTrack {
    pub fn prepare(&mut self) -> Result<(), StringError> {
        debug!("Preparing terminal");
        match unsafe {c::bis_prepare_terminal()} {
            0 => Ok(()),
            _ => Err(unsafe {c::get_bis_error()})
        }
    }

    pub fn restore(&mut self) -> Result<(), StringError> {
        debug!("Restoring terminal");
        match unsafe {c::bis_restore_terminal()} {
            0 => Ok(()),
            _ => Err(unsafe {c::get_bis_error()})
        }
    }

    pub fn get_size(&self) -> Result<TermSize, StringError> {
        debug!("Getting terminal size");
        let mut term_size = c::bis_term_size_t {
            rows: 0,
            cols: 0
        };

        match unsafe {c::bis_get_terminal_size(&mut term_size)} {
            0 => Ok(TermSize {
                rows: term_size.rows as usize,
                cols: term_size.cols as usize
            }),
            _ => Err(unsafe {c::get_bis_error()})
        }
    }
}

pub fn mask_sigint() -> Result<(), StringError> {
    debug!("Masking sigint");
    match unsafe {c::bis_mask_sigint()} {
        0 => Ok(()),
        _ => Err(unsafe {c::get_bis_error()})
    }
}

pub fn wait_sigint() -> Result<(), StringError> {
    debug!("Waiting for sigint");
    match unsafe {c::bis_wait_sigint()} {
        0 => Ok(()),
        _ => Err(unsafe {c::get_bis_error()})
    }
}

pub fn insert_input<T: Into<Vec<u8>>>(input: T) -> Result<(), StringError> {
    let cstr = match CString::new(input) {
        Ok(s) => s,
        Err(e) => return Err(StringError::new("Failed to create CString", Some(Box::new(e))))
    };

    match unsafe {c::bis_insert_input(cstr.as_ptr())} {
        0 => {},
        _ => return Err(unsafe {c::get_bis_error()})
    }

    // return success
    Ok(())
}
