// process spawning process:
use libc::*;

use std::os::unix::prelude::*;
use std::io::{Read, Write};

use std::io;
use std::ffi;
use std::ptr;
use std::mem;
use std::os;
use std::slice;

use constants::*;
use util::*;
use signal::*;

pub struct OsPipe(Fd);

#[derive(Clone)]
pub struct Socket(OsPipe);

#[derive(Copy)]
pub enum ExitStatus {
    Code(c_int),
    Signal(c_int)
}

#[derive(Copy)]
pub enum Fork {
    Child,
    Parent(pid_t)
}

pub enum Message {
    FDs(Vec<Fd>),
    Other
}

#[derive(Clone)]
pub struct StandardOutput {
    pub stdout: String,
    pub stderr: String
}

pub struct Process {
    pid: Option<pid_t>,
    file: ffi::OsString,
    args: Vec<ffi::OsString>,
    pub stdin: Option<OsPipe>,
    pub stdout: Option<OsPipe>,
    pub stderr: Option<OsPipe>
}

mod c {
    use libc::{c_int, c_void, size_t, ssize_t,
               socklen_t, c_uchar};
    use std::os::unix::prelude::*;
    use signal::*;
    use std::raw::{self, Repr};

    #[repr(C)]
    pub struct iovec {
        pub base: *const c_void,
        pub len: size_t
    }

    impl iovec {
        pub unsafe fn from_slice(slice:&mut [u8]) -> iovec {
            let repr = slice.repr();
            iovec {
                base: repr.data as *mut c_void,
                len: repr.len as size_t
            }
        }
    }

    #[repr(C)]
    pub struct msghdr {
        pub name: *mut c_void,
        pub namelen: socklen_t,
        pub iov: *mut iovec,
        pub iovlen: size_t,
        pub control: *mut c_void,
        pub controllen: size_t,
        pub flags: c_int
    }

    // msghdr's control had a concept of messages, which are prefixed
    // by these headers. These messages are aligned to size_t.
    #[repr(C)]
    pub struct cmsghdr {
        pub len: size_t,
        pub level: c_int,
        pub mtype: c_int
    }

    #[link(name="c")]
    extern {
        pub fn pipe2(pipefd: *mut Fd, flags: c_int) -> c_int;
        pub fn dup3(oldfd: Fd, newfd: Fd, flags: c_int) -> c_int;
        pub fn fcntl(fd: Fd, cmd: c_int, ...) -> c_int;
        pub fn socketpair(domain: c_int, socket_type: c_int, protocol: c_int, sv: *mut Fd) -> c_int;
        pub fn sendmsg(sockfd: Fd, msg: *const msghdr, flags: c_int) -> ssize_t;
        pub fn recvmsg(sockfd: Fd, msg: *mut msghdr, flags: c_int) -> ssize_t;
    }
}

impl ExitStatus {
    pub fn success(&self) -> bool {
        match self {
            &ExitStatus::Code(0) => true,
            _ => false
        }
    }
}

impl Process {
    pub fn pipe(file: String, args: Vec<String>,
                stdin: Option<OsPipe>, stdout: Option<OsPipe>,
                stderr: Option<OsPipe>) -> Process {
        Process {
            pid: None,
            file: ffi::OsString::from_string(file),
            args: args.clone().into_iter().map(|s| {ffi::OsString::from_string(s)}).collect(),
            stdin: stdin, stdout: stdout, stderr: stderr
        }
    }

    pub fn connect(file: String, args: Vec<String>) -> Process {
        Process::pipe(file, args, None, None, None)
    }

    pub fn signal(&self, signal: c_int, value: Option<SigVal>) -> io::Result<()> {
        match self.pid {
            None => Err(io::Error::new(io::ErrorKind::Other, "process not spawned", None)),
            Some(pid) => match value {
                None => send_signal(pid, signal, SigVal::empty()),
                Some(val) => send_signal(pid, signal, val)
            }
        }
    }

    #[inline]
    pub fn pid(&self) -> Option<pid_t> {
        self.pid
    }

    pub fn read_output(&mut self) -> io::Result<StandardOutput> {
        // read-to-string on stdout, stderr
        // makes assumptions, could cause deadlock, but this isn't "unsafe" by Rust
        // standards
        let mut stdout = vec![];
        let mut stderr = vec![];
        match self.stdout {
            Some(ref mut pipe) => {
                try!(pipe.read_to_end(&mut stdout));
            },
            None => {}
        }
        match self.stderr {
            Some(ref mut pipe) => {
                try!(pipe.read_to_end(&mut stderr));
            },
            None => {}
        }
        Ok(StandardOutput {
            stdout: String::from_utf8_lossy(stdout.as_slice()).into_owned(),
            stderr: String::from_utf8_lossy(stderr.as_slice()).into_owned()
        })
    }

    // spawn is unsafe because it can leak resources in some situations
    // use it with care

    #[inline]
    pub unsafe fn spawn(&mut self) -> io::Result<pid_t> {
        self.spawn_hook(|| {})
    }

    pub unsafe fn spawn_hook<T:FnOnce()>(&mut self, child_hook:T) -> io::Result<pid_t> {
        // Assumptions are being made in this function.
        // An issue with Linux is that there is no spawn function, in other words,
        // the only way to start a new process is fork and exec.
        // The problem with this is that fork creates a copy of the process, which
        // includes all the resources that have been allocated. As a result, it's possible
        // to leak memory into the child if it is not carefully discarded before the exec
        // happens. This is really hard to in Rust in a "correct" way, so the best solution
        // is to have a separate process which deals with process spawning,
        // which is relatively small and simple, and easy to audit for memory leaks.
        // Right now this code sits in here because it's an easy place to develop it,
        // once this code is semi-working it will be moved into a separate binary, which
        // will be started with Rusts's process management tools. That way, the side
        // where speed matters can be custom-built to be simple and fast, and the side
        // that deals directly with the hugeness of Rust's runtime can be simple in
        // implementation, and still relatively safe.

        // create sync pipes which will close on exec
        let (mut output, mut input) = try!(OsPipe::pair(Some(O_CLOEXEC)));

        match try!(fork_process()) {
            Fork::Child => {
                // drop the output pipe, we won't use it
                mem::drop(output);
                // child routine
                self.spawn_child(child_hook, &mut input);
            }
            Fork::Parent(pid) => {
                // drop the input pipe, we won't use it
                mem::drop(input);
                let mut bytes = [0; 4];

                // We need a way to be sure that a process is done
                // We do this by setting CLOEXEC on the pipes, so that once the child
                // calls exec, it will close the pipes
                // Because of the Drop trait on OsPipe, any panics will also close the
                // pipe
                // As a result, all the exit paths from our code in the child *should*
                // result in the pipe being closed
                // Additionally, if we run into any I/O errors, the errno is sent over
                // the pipe. This avoids a race condition.

                loop {
                    match output.read(&mut bytes) {
                        Ok(0) => {
                            // pipe closed: exec happened
                            self.pid = Some(pid);
                            return Ok(pid);
                        },
                        // i32 is four bytes long
                        Ok(4) => {
                            return Err(io::Error::from_os_error(*bytes_to_i32(&bytes)));
                        },
                        Ok(_) => {
                            panic!("Short read on spawn pipe");
                        },
                        Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                            // our read was interrupted, try again
                        },
                        Err(e) => {
                            // we should probably panic here
                            panic!("Failed to read from spawn pipe: {}", e);
                        }
                    }
                }
            }
        }
    }

    unsafe fn spawn_child<T:FnOnce()>(&mut self, hook:T, input:&mut Write) -> ! {
        // TODO: find a better way to pass errors back to the parent than this
        // child spawn subroutine
        // in a separate function because
        //   a) it's long
        //   b) that way we can ensure it is bottomless
        // create list of pointers to arguments
        // constructed like execvp wants
        let mut ptrs:Vec<*const c_char> = Vec::with_capacity(self.args.len()+1);
        let mut args_cstrs:Vec<ffi::CString> = Vec::with_capacity(self.args.len());
        let file_cstr = match self.file.as_os_str().to_cstring() {
            Ok(cstr) => cstr,
            Err(e) => {
                tryp!(input.write(&[0]));
                panic!("Could not get file cstring: {}", e);
            }
        };
        for arg in self.args.iter() {
            args_cstrs.push(match arg.as_os_str().to_cstring() {
                Ok(cstr) => cstr,
                Err(e) => {
                    tryp!(input.write(&[0]));
                    panic!("Could not get argument {:?} cstring: {}", arg, e);
                }
            });
        }
        // push file (command) as first argument
        ptrs.push(file_cstr.as_ptr());
        // push arguments next
        for arg in args_cstrs {
            ptrs.push(arg.as_ptr());
        }
        // finish with null pointer
        ptrs.push(ptr::null());
        // set an empty signal mask
        let sigset = match empty_sigset() {
            Err(e) => {
                match e.raw_os_error() {
                    Some(ref code) => {tryp!(input.write(i32_to_bytes(code)));},
                    None => {tryp!(input.write(&[0]));}
                }
                panic!("Failed to get empty sigset: {}", e);
            },
            Ok(set) => set
        };
        match signal_proc_mask(SIG_SETMASK, &sigset) {
            Err(e) => {
                match e.raw_os_error() {
                    Some(ref code) => {tryp!(input.write(i32_to_bytes(code)));},
                    None => {tryp!(input.write(&[0]));}
                }
                panic!("Failed to set signal mask: {}", e);
            },
            Ok(_) => {}
        }

        // handle descriptors
        match self.stdin {
            Some(ref mut pipe) => match pipe.ensure_at(STDIN, None) {
                Err(e) => {
                    match e.raw_os_error() {
                        Some(ref code) => {tryp!(input.write(i32_to_bytes(code)));},
                        None => {tryp!(input.write(&[0]));}
                    }
                    panic!("Failed to set stdin: {}", e);
                },
                Ok(_) => {}
            },
            None => {}
        }
        
        match self.stdout {
            Some(ref mut pipe) => match pipe.ensure_at(STDOUT, None) {
                Err(e) => {
                    match e.raw_os_error() {
                        Some(ref code) => {tryp!(input.write(i32_to_bytes(code)));},
                        None => {tryp!(input.write(&[0]));}
                    }
                    panic!("Failed to set stdout: {}", e);
                },
                Ok(_) => {}
            },
            None => {}
        }
        
        match self.stderr {
            Some(ref mut pipe) => match pipe.ensure_at(STDERR, None) {
                Err(e) => {
                    match e.raw_os_error() {
                        Some(ref code) => {tryp!(input.write(i32_to_bytes(code)));},
                        None => {tryp!(input.write(&[0]));}
                    }
                    panic!("Failed to set stderr: {}", e);
                },
                Ok(_) => {}
            },
            None => {}
        }

        // TODO: maybe support setting different environment variables and
        // the such

        // run child hook
        hook();

        // Replace the process
        // closes input and output pipe
        execvp(file_cstr.as_ptr(), ptrs.as_mut_ptr());

        // Fail
        tryp!(input.write(i32_to_bytes(&os::errno())));
        panic!("exec failed: {}", io::Error::last_os_error());
    }
}

impl Socket {
    pub fn new(fd: Fd) -> Socket {
        Socket(OsPipe::new(fd))
    }

    #[inline]
    pub fn raw(&self) -> &OsPipe {
        &self.0
    }

    #[inline]
    pub fn close(&mut self) -> io::Result<()> {
        self.0.close()
    }

    #[inline]
    pub fn ensure_at(&mut self, fd: Fd, flags: Option<c_int>) -> io::Result<()> {
        self.0.ensure_at(fd, flags)
    }

    #[inline]
    pub fn set_cloexec(&mut self) -> io::Result<()> {
        self.0.set_cloexec()
    }

    pub fn pair(domain: c_int, socket_type: c_int,
                protocol: c_int) -> io::Result<(Socket, Socket)> {
        let mut sv = [0; 2];
        match unsafe {c::socketpair(domain, socket_type, protocol, sv.as_mut_ptr())} {
            0 => Ok((Socket::new(sv[0]), Socket::new(sv[1]))),
            _ => Err(io::Error::last_os_error())
        }
    }

    // These functions have to be here for lifetime issues: the buffers used
    // to contain the data from the reads must outlive the data structures
    // that point to them

    pub fn send_msg(&mut self, buf: &[u8]) -> io::Result<()> {
        // send the magic value corresponding to another type of message
        // this is to tell gamete to enter the other read loop, and wait for
        // other messages
        // sock_stream means that a short read doesn't drop data
        // so we can send these together, and the client can then read
        // the data in a separate syscall afterwards.
        let magic = MAGIC_MSG;
        let magic_buf = u64_to_bytes(&magic);
        let mut combined_slice = vec![magic_buf, buf].concat();
        let mut iov = unsafe {c::iovec::from_slice(&mut combined_slice)};
        // Maybe one day we'll care about using iovecs, but for now
        // these messages are just useful to pass file descriptors around
        let message = c::msghdr {
            name: ptr::null_mut(),
            namelen: 0,
            iov: &mut iov,
            iovlen: 1,
            control: ptr::null_mut(),
            controllen: 0,
            flags: 0
        };
        // now send the message
        match unsafe {c::sendmsg(self.0.raw(), &message, 0)} {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(())
        }
    }

    pub fn send_fds(&mut self, fds: Vec<Fd>) -> io::Result<()> {
        // A number of assumptions are made in this function
        // The bottom line is that this code is too special-use to
        // warrant a more general implementation
        // first create the control buffer
        let len = align_len(mem::size_of::<c::cmsghdr>(), mem::size_of::<size_t>()) +
            fds.len()*mem::size_of::<Fd>();
        let size = align_len(len, mem::size_of::<size_t>());
        if size > MAX_CONTROL_SIZE {
            return Err(io::Error::new(io::ErrorKind::Other, "control message too long",
                                      Some(format!("Control messages must be no longer than 64 bytes, was {}", size))))
        }
        let mut cheader = c::cmsghdr {
            len: len as size_t,
            level: SOL_SOCKET,
            mtype: SCM_RIGHTS
        };
        // Create a separate buffer first so that Rust doesn't shit the bed
        let mut buf = Vec::with_capacity(size);
        // "How do we get a byte buffer out of these?"
        // Well, just coerce things using from_raw_parts
        let cslice = unsafe {slice::from_raw_parts::<u8>(
            &cheader as *const _ as *const u8, // transmute cheader to u8
            mem::size_of::<c::cmsghdr>()/mem::size_of::<u8>())};
        buf.push_all(cslice);
        let fdslice = unsafe {slice::from_raw_parts::<u8>(
            fds.as_slice().as_ptr() as *const u8,
            fds.len() / mem::size_of::<u8>())};
        buf.push_all(fdslice);
        assert!(size >= buf.len());
        for _ in (0 .. size - buf.len()) {
            buf.push(0 as u8);
        }
        assert!(size == buf.len());
        // we *have* to send a message with this, otherwise the write fails
        // so just send a null byte
        let mut magic_buf = [0; 8];
        let magic = MAGIC_FD;
        let bytes = u64_to_bytes(&magic);
        for i in (0..8) {
            magic_buf[i] = bytes[i];
        }
        let mut iov = unsafe {c::iovec::from_slice(&mut magic_buf)};
        // Maybe one day we'll care about using iovecs, but for now
        // these messages are just useful to pass file descriptors around
        let message = c::msghdr {
            name: ptr::null_mut(),
            namelen: 0,
            iov: &mut iov,
            iovlen: 1,
            control: buf.as_mut_slice().as_mut_ptr() as *mut _,
            controllen: size as size_t,
            flags: 0
        };
        // now send the message
        match unsafe {c::sendmsg(self.0.raw(), &message, 0)} {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(())
        }
    }

    pub fn receive_msg(&mut self) -> io::Result<Message> {
        // This function can do a list, but it only deals with the first cmsg header
        // Any following ones are ignored
        let mut buffer = [0; MAX_CONTROL_SIZE];
        let mut magic_buf = [0; 8];
        // TODO: use a magic number
        let mut iov = unsafe {c::iovec::from_slice(&mut magic_buf)};
        let mut message = c::msghdr {
            name: ptr::null_mut(),
            namelen: 0,
            iov: &mut iov,
            iovlen: 1,
            control: buffer.as_mut_ptr() as *mut _,
            controllen: MAX_CONTROL_SIZE as size_t,
            flags: 0
        };
        match unsafe {c::recvmsg(self.0.raw(), &mut message, 0)} {
            -1 => return Err(io::Error::last_os_error()),
            0 => {
                // pipe was closed
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "socket read no bytes", None))
            },
            8 => {/* read the null byte, continue */}
            l => panic!("Incorrect read length: {}", l)
        }
        // check for magic
        match *bytes_to_u64(&magic_buf) {
            MAGIC_FD => {
                // FD
                // check for truncated messages
                if message.flags & MSG_CTRUNC != 0 {
                    panic!("Control buffer was not long enough");
                }
                // ignore everything but control
                if message.controllen < mem::size_of::<c::cmsghdr>() as size_t {
                    return Err(io::Error::new(io::ErrorKind::Other, "control data not long enough",
                                              Some(format!("Was: {}, Should be at least: {}",
                                                           message.controllen,
                                                           mem::size_of::<c::cmsghdr>()))));
                }
                if message.control.is_null() {
                    return Err(io::Error::new(io::ErrorKind::Other, "control message pointer was null", None));
                }
                // only treat the first header
                let header = unsafe {(message.control as *const c::cmsghdr).as_ref()}.unwrap();
                assert!(header.len <= message.controllen);
                // pointer arithmetic FTW
                let data_ptr = unsafe {(message.control as *mut c::cmsghdr).offset(1)} as *mut Fd;
                let len = (header.len as usize - align_len(mem::size_of::<c::cmsghdr>(), mem::size_of::<size_t>()))/
                    mem::size_of::<Fd>();
                Ok(Message::FDs(unsafe {Vec::from_raw_parts(data_ptr, len, len)}))
            },
            MAGIC_MSG => {
                // some other message
                Ok(Message::Other)
            },
            n => {
                // unknown message
                Err(io::Error::new(io::ErrorKind::Other, "got unknown magic",
                                   Some(format!("Got: {}", n))))
            }
        }
    }
}

impl OsPipe {
    pub fn new(fd: Fd) -> OsPipe {
        OsPipe(fd)
    }

    pub fn pair(flags: Option<c_int>) -> io::Result<(OsPipe, OsPipe)> {
        let mut fds:[Fd; 2] = [0; 2];
        match flags {
            Some(f) => match unsafe {c::pipe2(fds.as_mut_ptr(), f)} {
                0 => Ok((OsPipe::new(fds[0]), OsPipe::new(fds[1]))),
                _ => Err(io::Error::last_os_error())
            },
            None => match unsafe {pipe(fds.as_mut_ptr())} {
                0 => Ok((OsPipe::new(fds[0]), OsPipe::new(fds[1]))),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    #[inline]
    pub fn raw(&self) -> Fd {
        self.0
    }

    pub fn set_cloexec(&mut self) -> io::Result<()> {
        match unsafe {fcntl(self.0, F_SETFD, FD_CLOEXEC)} {
            0 => Ok(()),
            _ => Err(io::Error::last_os_error())
        }
    }

    pub fn close(&mut self) -> io::Result<()> {
        match unsafe {close(self.0)} {
            0 => Ok(()),
            _ => Err(io::Error::last_os_error())
        }
    }

    #[inline]
    pub fn write_char(&mut self, c: char) -> io::Result<()> {
        let mut buf = [0; 4];
        let n = c.encode_utf8(&mut buf).unwrap_or(0);
        self.write_all(&buf[..n])
    }

    #[inline]
    pub fn write_str(&mut self, s: &str) -> io::Result<()> {
        self.write_all(s.as_bytes())
    }

    pub fn duplicate(&self, new: Option<OsPipe>, flags: Option<c_int>) -> io::Result<OsPipe> {
        match new {
            Some(newpipe) => match flags {
                Some(f) => match unsafe {c::dup3(self.0, newpipe.0, f)} {
                    -1 => Err(io::Error::last_os_error()),
                    ref fid if *fid == newpipe.0 => Ok(newpipe),
                    fid => panic!("dup2 didn't return the new descriptor it was given: {}", fid)
                },
                None => match unsafe {dup2(self.0, newpipe.0)} {
                    -1 => Err(io::Error::last_os_error()),
                    ref fid if *fid == newpipe.0 => Ok(newpipe),
                    fid => panic!("dup2 didn't return the new descriptor it was given: {}", fid)
                }
            },
            None => match unsafe {dup(self.0)} {
                -1 => Err(io::Error::last_os_error()),
                fid => Ok(OsPipe::new(fid))
            }
        }
    }

    pub fn ensure_at(&mut self, fd: Fd, flags: Option<c_int>) -> io::Result<()> {
        if fd == self.0 {
            Ok(())
        } else {
            match self.duplicate(Some(OsPipe::new(fd)), flags) {
                Ok(pipe) => {
                    self.close();
                    *self = pipe;
                    Ok(())
                },
                Err(e) => Err(e)
            }
        }
    }
}

impl Clone for OsPipe {
    fn clone(&self) -> OsPipe {
        match self.duplicate(None, None) {
            Err(e) => panic!("Could not duplicate pipe: {}", e),
            Ok(pipe) => pipe
        }
    }

    fn clone_from(&mut self, source: &OsPipe) {
        *self = source.clone();
    }
}

impl Drop for OsPipe {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        // ignore errors
        if self.0 > 2 {
            // don't close stdio pipes
            self.close();
        }
    }
}

impl io::Read for OsPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = buf.len();
        match unsafe {read(self.0, buf.as_mut_ptr() as *mut _, count as size_t)} {
            -1 => Err(io::Error::last_os_error()),
            num => Ok(num as usize)
        }
    }
}

impl io::Read for Socket {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl io::Write for OsPipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let count = buf.len();
        match unsafe {write(self.0, buf.as_ptr() as *const _, count as size_t)} {
            -1 => Err(io::Error::last_os_error()),
            num => Ok(num as usize)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match unsafe {fsync(self.0)} {
            0 => Ok(()),
            _ => Err(io::Error::last_os_error())
        }
    }
}

impl io::Write for Socket {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

pub fn fork_process() -> io::Result<Fork> {
    let pid = unsafe{fork()};
    if pid < 0 {
        Err(io::Error::last_os_error())
    } else if pid == 0 {
        Ok(Fork::Child)
    } else {
        Ok(Fork::Parent(pid))
    }
}
