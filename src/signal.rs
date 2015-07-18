use libc::*;

use std::os::unix::prelude::*;

use std::io;
use std::mem;
use std::slice;

use constants::*;

#[derive(Copy)]
pub struct SigVal {
    // _data is either a c_int (4 bytes), or a *const c_void (8 bytes)
    _data: [c_int; SI_VAL_SIZE]
}

impl SigVal {
    pub fn empty() -> SigVal {
        SigVal {
            _data: [0; SI_VAL_SIZE]
        }
    }

    pub fn from_int(val:c_int) -> SigVal {
        SigVal {
            _data: [val, 0]
        }
    }

    pub fn from_ptr<T>(ptr:*const T) -> SigVal {
        let data = unsafe {slice::from_raw_parts([ptr].as_ptr() as *const c_int, 2)};
        SigVal {
            _data: [data[0], data[1]]
        }
    }
    
    pub fn as_int(&self) -> c_int {
        self._data[0]
    }

    pub fn as_ptr(&self) -> *const c_void {
        unsafe {slice::from_raw_parts::<*const c_void>(self._data.as_ptr() as *const _, 1)[0]}
    }
}

// The size_t at the end is a pointer to a ucontext_t
// do I want to implement that type? Lol no
// Protip: we're calling sigaction with SA_SIGINFO
// In part because I didn't realize I didn't have to implement
// all of SigInfo, so I did, and now I want to use that code dammit!
// SigHandler is by definition unsafe, note it as so
pub type SigHandler = unsafe extern fn(c_int, *const SigInfo, *const c_void);
pub type SigSet = [c_ulong; SIGSET_NWORDS];
type FdSet = [c_long; FD_SET_SIZE];

// "What is all this?"
// This is an implementation of C unions in Rust

#[derive(Copy)]
pub struct KillFields {
    pub pid: pid_t,
    pub uid: uid_t,
    _pad: [c_int; (SI_PAD_SIZE - 2)]
}

#[derive(Copy)]
pub struct PTimerFields {
    pub tid: c_int,
    pub overrun: c_int,
    pub sigval: SigVal,
    _pad: [c_int; (SI_PAD_SIZE - 4)]
}

#[derive(Copy)]
pub struct PSignalFields {
    pub pid: pid_t,
    pub uid: uid_t,
    pub sigval: SigVal,
    _pad: [c_int; (SI_PAD_SIZE - 4)]
}

#[derive(Copy)]
pub struct SigChldFields {
    pub pid: pid_t,
    pub uid: uid_t,
    pub status: c_int,
    pub utime: clock_t,
    pub stime: clock_t,
    _pad: [c_int; (SI_PAD_SIZE - 8)]
}

#[derive(Copy)]
pub struct SigFaultFields {
    pub addr: size_t, // pointer
    pub addr_lsb: c_short,
    _pad: [c_int; (SI_PAD_SIZE - 3)]
}

#[derive(Copy)]
pub struct SigPollFields {
    pub band: c_long,
    pub fd: c_int,
    _pad: [c_int; (SI_PAD_SIZE - 3)]
}

#[derive(Copy)]
pub struct SigSysFields {
    _call_addr: size_t, // pointer
    _syscall: c_int,
    _arch: c_uint,
    _pad: [c_int; (SI_PAD_SIZE - 4)]
}

#[derive(Copy)]
pub enum SigFields {
    Kill(KillFields),
    PTimer(PTimerFields),
    PSignal(PSignalFields),
    SigChld(SigChldFields),
    SigFault(SigFaultFields),
    SigPoll(SigPollFields),
    SigSys(SigSysFields)
}

#[repr(C)]
#[derive(Copy)]
pub struct SigInfo {
    pub signo: c_int,
    pub errno: c_int,
    pub code: c_int,

    // Rust has no struct alignment, so manual padding is necessary
    _align: [c_int; SI_PREAMBLE - 3],

    // Generic memory pad for union contents
    _data: [c_int; SI_PAD_SIZE]
}

#[repr(C)]
#[derive(Copy)]
pub struct SigAction {
    pub handler: SigHandler,
    pub mask: SigSet,
    pub flags: c_int,
    // This is technically a pointer, but the man pages say to keep it null,
    // and that's easier if we just use a size_t
    pub restorer: size_t
}

impl SigAction {
    pub fn new() -> SigAction {
        SigAction {
            handler: unsafe {mem::transmute::<usize, SigHandler>(0)},
            mask: full_sigset().unwrap_or([0; SIGSET_NWORDS]),
            flags: SA_RESTART,
            restorer: 0
        }
    }

    pub fn ignore() -> SigAction {
        SigAction {
            handler: unsafe {mem::transmute::<usize, SigHandler>(1)},
            mask: full_sigset().unwrap_or([0; SIGSET_NWORDS]),
            flags: SA_RESTART,
            restorer: 0
        }
    }

    pub fn handler(handler:SigHandler) -> SigAction {
        SigAction {
            handler: handler,
            mask: full_sigset().unwrap_or([0; SIGSET_NWORDS]),
            flags: SA_RESTART | SA_SIGINFO,
            restorer: 0
        }
    }
}

impl SigInfo {
    pub fn new() -> SigInfo {
        SigInfo {
            signo: 0,
            errno: 0,
            code: 0,
            _align: [0; SI_PREAMBLE - 3],
            _data: [0; SI_PAD_SIZE]
        }
    }

    pub fn determine_sigfields(&self) -> SigFields {
        unsafe {
            match self.signo {
                SIGKILL => SigFields::Kill(mem::transmute(self._data)),
                SIGALRM | SIGVTALRM | SIGPROF => SigFields::PTimer(mem::transmute(self._data)),
                SIGCHLD => SigFields::SigChld(mem::transmute(self._data)),
                SIGILL | SIGFPE | SIGSEGV | SIGBUS => SigFields::SigFault(mem::transmute(self._data)),
                SIGPOLL => SigFields::SigPoll(mem::transmute(self._data)),
                SIGSYS => SigFields::SigSys(mem::transmute(self._data)),
                _ => SigFields::PSignal(mem::transmute(self._data))
            }
        }
    }
}

#[link(name="c")]
extern {
    fn sigaction(signum:c_int, act:*const SigAction, oldact:*mut SigAction) -> c_int;
    fn sigfillset(set:*mut SigSet) -> c_int;
    fn sigemptyset(set:*mut SigSet) -> c_int;
    fn sigaddset(set:*mut SigSet, signal:c_int) -> c_int;
    fn sigwaitinfo(set:*const SigSet, info:*mut SigInfo) -> c_int;
    fn sigtimedwait(set:*const SigSet, info:*mut SigInfo, timeout:*const timespec) -> c_int;
    fn signalfd(fd:Fd, mask:*const SigSet, flags:c_int) -> c_int;
    fn pselect(nfds:c_int, readfds:*mut FdSet, writefds:*mut FdSet,
               exceptfds:*mut FdSet, timeout:*const timespec, sigmask:*const SigSet) -> c_int;
    fn sigprocmask(how:c_int, set:*const SigSet, old_set:*mut SigSet) -> c_int;
    fn sigqueue(pid:pid_t, sig:c_int, value:SigVal) -> c_int;
}

pub fn signal_proc_mask(how:c_int, set:&SigSet) -> io::Result<SigSet> {
    let mut old_set = try!(empty_sigset());
    match unsafe {sigprocmask(how, set, &mut old_set)} {
        0 => Ok(old_set),
        _ => Err(io::Error::last_os_error())
    }
}

fn fd_set_empty() -> FdSet {
    return [0; FD_SET_SIZE];
}

fn fd_set_add(fd:Fd, set:&mut FdSet) {
    let index = fd as usize / NFD_BITS;
    let mask = 1 << (fd % NFD_BITS as i32);
    set[index] |= mask;
}

// this function needs to be here for completeness sake
#[allow(dead_code)]
fn fd_set_rm(fd:Fd, set:&mut FdSet) {
    let index = fd as usize / NFD_BITS;
    let mask = 1 << (fd % NFD_BITS as i32);
    set[index] &= !mask;
}

fn fd_is_set(fd:Fd, set:&FdSet) -> bool {
    let index = fd as usize / NFD_BITS;
    let mask = 1 << (fd % NFD_BITS as i32);
    return set[index] & mask != 0;
}

pub fn select(read:&Vec<Fd>, write:&Vec<Fd>, except:&Vec<Fd>,
              timeout:Option<usize>, sigmask:&SigSet) -> io::Result<Vec<Fd>> {
    let mut readfds = fd_set_empty();
    let mut writefds = fd_set_empty();
    let mut exceptfds = fd_set_empty();
    let mut max = 0;
    for fd in read.iter() {
        if *fd > max {
            max = *fd;
        }
        fd_set_add(*fd, &mut readfds);
    }
    for fd in write.iter() {
        if *fd > max {
            max = *fd;
        }
        fd_set_add(*fd, &mut writefds);
    }
    for fd in except.iter() {
        if *fd > max {
            max = *fd;
        }
        fd_set_add(*fd, &mut exceptfds);
    }
    match match timeout {
        None => unsafe {pselect(max + 1, &mut readfds, &mut writefds,
                                &mut exceptfds, 0 as *const timespec,
                                sigmask)},
        Some(t) => {
            let time = timespec {
                tv_sec: (t / 1000) as c_longlong,
                tv_nsec: ((t % 1000) * 1000) as c_long
            };
            unsafe {pselect(max + 1, &mut readfds, &mut writefds,
                            &mut exceptfds, &time, sigmask)}
        }
    } {
        -1 => return Err(io::Error::last_os_error()),
        v => {
            let mut out = vec![];
            for fd in read.iter() {
                if fd_is_set(*fd, &mut readfds) {
                    out.push(*fd);
                }
            }
            for fd in write.iter() {
                if fd_is_set(*fd, &mut writefds) {
                    out.push(*fd);
                }
            }
            for fd in except.iter() {
                if fd_is_set(*fd, &mut exceptfds) {
                    out.push(*fd);
                }
            }
            if out.len() != v as usize {
                panic!("Too many file descriptors set after select call");
            }
            return Ok(out);
        }
    }
}

pub fn signal_fd(set:&SigSet) -> io::Result<Fd> {
    match unsafe {signalfd(-1, set, 0)} {
        -1 => Err(io::Error::last_os_error()),
        fd => Ok(fd)
    }
}

pub fn signal_wait_set(set:&SigSet, timeout:Option<usize>) -> io::Result<SigInfo> {
    let mut info = SigInfo::new();
    match timeout {
        None => match unsafe {sigwaitinfo(set, &mut info)} {
            v if v > 0 => Ok(info),
            _ => Err(io::Error::last_os_error())
        },
        Some(t) => {
            let time = timespec {
                tv_sec: (t / 1000) as c_longlong,
                tv_nsec: ((t % 1000) * 1000) as c_long
            };
            match unsafe {sigtimedwait(set, &mut info, &time)} {
                v if v > 0 => Ok(info),
                _ => Err(io::Error::last_os_error())
            }
        }
    }
}

pub fn signal_handle(signal:c_int, action:*const SigAction) -> io::Result<SigAction> {
    unsafe {
        let old_act = &mut SigAction::new();
        match sigaction(signal, action, old_act) {
            0 => Ok(*old_act),
            _ => Err(io::Error::last_os_error())
        }
    }
}

pub fn signal_ignore(signal:c_int) -> io::Result<SigAction> {
    let action = SigAction::ignore();
    signal_handle(signal, &action)
}

pub fn signal_default(signal:c_int) -> io::Result<SigAction> {
    let action = SigAction::new();
    signal_handle(signal, &action)
}

pub fn full_sigset() -> io::Result<SigSet> {
    let mut output:SigSet = [0; SIGSET_NWORDS];
    unsafe {
        match sigfillset(&mut output) {
            0 => Ok(output),
            _ => Err(io::Error::last_os_error())
        }
    }
}

pub fn empty_sigset() -> io::Result<SigSet> {
    let mut output:SigSet = [0; SIGSET_NWORDS];
    unsafe {
        match sigemptyset(&mut output) {
            0 => Ok(output),
            _ => Err(io::Error::last_os_error())
        }
    }
}

pub fn sigset_add(set:&mut SigSet, signal:c_int) -> io::Result<()> {
    unsafe {
        match sigaddset(set, signal) {
            0 => Ok(()),
            _ => Err(io::Error::last_os_error())
        }
    }
}

pub fn send_signal(pid:pid_t, signal:c_int, value:SigVal) -> io::Result<()> {
    match unsafe {sigqueue(pid, signal, value)} {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error())
    }
}
