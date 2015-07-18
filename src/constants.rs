use libc::*;
use regex::Regex;

// Settings which will eventually make their way into some type of
// configuration interface
pub const HISTORY_SIZE:usize = 500;

// standard file descriptiors
pub const STDIN:c_int  = 0;
pub const STDOUT:c_int = 1;
pub const STDERR:c_int = 2;

// file descriptors used for IPC
pub const SPAWN_PIPE:c_int = 405;

pub const NFD_BITS:usize = 8 * 8; // 8 * sizeof(long int)
pub const FD_SET_SIZE:usize = 1024 / NFD_BITS;
pub const MAX_CONTROL_SIZE:usize = 64;

// constants for control sequences is useful
pub const CEOF:char = '\u{4}';
pub const DEL:char = '\u{7f}';
pub const NL:char  = '\n';
pub const ESC:char = '\u{1b}';
pub const ANSI:char = '\u{5b}'; // [
pub const BS:char = '\u{8}';
pub const SPC:char = ' ';
pub const CMA:char = ',';
pub const QUT:char = '\"';
pub const OPR:char = '\u{28}'; // (
pub const CPR:char = '\u{29}'; // )
pub const BEL:char = '\u{7}';
pub const CTA:char = '\u{1}';
pub const CTE:char = '\u{5}';
pub const CTK:char = '\u{b}';
// we specifically need a constant for "cursor right"
pub const CRSR_RIGHT:&'static str = "\u{1b}\u{5b}C";
pub const ANSI_BEGIN:&'static str = "\u{1b}\u{5b}";
pub const CRSR_POS:&'static str = "\u{1b}\u{5b}6n";

// select termios constants that we use
pub const ICANON:c_uint   = 2;
pub const ECHO:c_uint     = 8;
pub const TCSANOW:c_int   = 0;
pub const TCSADRAIN:c_int = 1;
pub const TCSAFLUSH:c_int = 2;

// signal values
//pub const SIGHUP:c_int    = 1;       /* Hangup (POSIX).  */
//pub const SIGINT:c_int    = 2;       /* Interrupt (ANSI).  */
//pub const SIGQUIT:c_int   = 3;       /* Quit (POSIX).  */
//pub const SIGILL:c_int    = 4;       /* Illegal instruction (ANSI).  */
//pub const SIGTRAP:c_int   = 5;       /* Trace trap (POSIX).  */
//pub const SIGABRT:c_int   = 6;       /* Abort (ANSI).  */
pub const SIGIOT:c_int    = 6;       /* IOT trap (4.2 BSD).  */
pub const SIGBUS:c_int    = 7;       /* BUS error (4.2 BSD).  */
//pub const SIGFPE:c_int    = 8;       /* Floating-point exception (ANSI).  */
//pub const SIGKILL:c_int   = 9;       /* Kill, unblockable (POSIX).  */
pub const SIGUSR1:c_int   = 10;      /* User-defined signal 1 (POSIX).  */
//pub const SIGSEGV:c_int   = 11;      /* Segmentation violation (ANSI).  */
pub const SIGUSR2:c_int   = 12;      /* User-defined signal 2 (POSIX).  */
//pub const SIGPIPE:c_int   = 13;      /* Broken pipe (POSIX).  */
//pub const SIGALRM:c_int   = 14;      /* Alarm clock (POSIX).  */
//pub const SIGTERM:c_int   = 15;      /* Termination (ANSI).  */
pub const SIGSTKFLT:c_int = 16;      /* Stack fault.  */
pub const SIGCLD:c_int    = SIGCHLD; /* Same as SIGCHLD (System V).  */
pub const SIGCHLD:c_int   = 17;      /* Child status has changed (POSIX).  */
pub const SIGCONT:c_int   = 18;      /* Continue (POSIX).  */
pub const SIGSTOP:c_int   = 19;      /* Stop, unblockable (POSIX).  */
pub const SIGTSTP:c_int   = 20;      /* Keyboard stop (POSIX).  */
pub const SIGTTIN:c_int   = 21;      /* Background read from tty (POSIX).  */
pub const SIGTTOU:c_int   = 22;      /* Background write to tty (POSIX).  */
pub const SIGURG:c_int    = 23;      /* Urgent condition on socket (4.2 BSD).  */
pub const SIGXCPU:c_int   = 24;      /* CPU limit exceeded (4.2 BSD).  */
pub const SIGXFSZ:c_int   = 25;      /* File size limit exceeded (4.2 BSD).  */
pub const SIGVTALRM:c_int = 26;      /* Virtual alarm clock (4.2 BSD).  */
pub const SIGPROF:c_int   = 27;      /* Profiling alarm clock (4.2 BSD).  */
pub const SIGWINCH:c_int  = 28;      /* Window size change (4.3 BSD, Sun).  */
pub const SIGPOLL:c_int   = SIGIO;   /* Pollable event occurred (System V).  */
pub const SIGIO:c_int     = 29;      /* I/O now possible (4.2 BSD).  */
pub const SIGPWR:c_int    = 30;      /* Power failure restart (System V).  */
pub const SIGSYS:c_int    = 31;      /* Bad system call.  */
pub const SIGUNUSED:c_int = 31;

// sizeof(int) = 4
// sizeof(unsigned long int) = 8
pub const SI_MAX_SIZE:usize = 128;
pub const SI_PREAMBLE:usize = 4;
pub const SI_PAD_SIZE:usize = (SI_MAX_SIZE / 4) - SI_PREAMBLE;
pub const SIGSET_NWORDS:usize = 16;
pub const SI_VAL_SIZE:usize = 2;

pub const SIG_BLOCK:c_int = 0;
pub const SIG_UNBLOCK:c_int = 1;
pub const SIG_SETMASK:c_int = 2;

pub const SA_NOCLDSTOP:c_int = 1;
pub const SA_NOCLDWAIT:c_int = 2;
pub const SA_SIGINFO:c_int   = 4;
pub const SA_RESTART:c_int   = 0x10000000;

pub const CLD_EXITED:c_int = 1;

pub const RTLD_LOCAL:c_int = 0;
pub const RTLD_LAZY:c_int = 1;

pub const WASH_RUN_SYMBOL:&'static str = "wash_run";
pub const WASH_LOAD_SYMBOL:&'static str = "wash_load";
pub const WO_PATH:&'static str = "/tmp/wash/";

pub const NCCS:usize = 32;

// ioctls
pub const TIOCGWINSZ:c_ulong = 0x00005413;

// regex
pub static VAR_REGEX:Regex = regex!("^\\$([^ \t\r\n\"():]+)");
pub static VAR_PATH_REGEX:Regex = regex!("^\\$([^ \t\r\n\"():]*):([^ \t\r\n\"():]*)");
pub static EQ_VAR_REGEX:Regex = regex!("^[^ \t\r\n\"()]+$");
pub static EQ_PATH_REGEX:Regex = regex!("^([^ \t\r\n\"():]*):([^ \t\r\n\"():]+)$");
pub static EQ_TEMP_REGEX:Regex = regex!("^([^ \t\r\n\"():]*):?([^ \t\r\n\"():]+):$");
pub static FD_REGEX:Regex = regex!("^@([^ \t\r\n\"():0123456789]*):?([\\d]+)$");
pub static PPOS_REGEX:Regex = regex!("^\\[(\\d+);(\\d+)$");
pub static NL_REGEX:Regex = regex!("\n");

// stop error
pub const STOP:&'static str = "stop";

pub const MAX_ESCAPE:usize = 15;

pub const HOST_NAME_MAX:usize = 64;

// strf needs an arbitrary buffer, hopefully 200 characters is big enough
pub const STRF_BUF_SIZE:usize = 200;

// fcntl constants

//pub const O_ACCMODE:c_int = 0o00000003;
//pub const O_RDONLY:c_int = 0o00000000;
//pub const O_WRONLY:c_int = 0o00000001;
//pub const O_RDWR:c_int = 0o00000002;
//pub const O_CREAT:c_int = 0o00000100;
//pub const O_EXCL:c_int = 0o00000200;
//pub const O_NOCTTY:c_int = 0o00000400;
//pub const O_TRUNC:c_int = 0o00001000;
//pub const O_APPEND:c_int = 0o00002000;
//pub const O_NONBLOCK:c_int = 0o00004000;
//pub const O_DSYNC:c_int = 0o00010000;
pub const FASYNC:c_int = 0o00020000;
pub const O_DIRECT:c_int = 0o00040000;
pub const O_LARGEFILE:c_int = 0o00100000;
pub const O_DIRECTORY:c_int = 0o00200000;
pub const O_NOFOLLOW:c_int = 0o00400000;
pub const O_NOATIME:c_int = 0o01000000;
pub const O_CLOEXEC:c_int = 0o02000000;

pub const FD_CLOEXEC:c_int = 1;
//pub const F_GETFD:c_int = 1;
//pub const F_SETFD:c_int = 2;

pub const SCM_RIGHTS:c_int = 1;
pub const SCM_CREDENTIALS:c_int = 2;
pub const SCM_SECURITY:c_int = 3;

// recvmsg msghdr flags
pub const MSG_EOR:c_int = 0x80;
pub const MSG_TRUNC:c_int = 0x20;
pub const MSG_CTRUNC:c_int = 0x8;
pub const MSG_OOB:c_int = 0x1;
pub const MSG_ERRQUEUE:c_int = 0x2000;

// magic values
pub const MAGIC_FD:u64  = 0xa84bf703c2213454;
pub const MAGIC_MSG:u64 = 0x2a1af12bca974468;
pub const MAGIC_CMD:u64 = 0xcaff36bec2703a24;

#[test]
fn fd_regex_test() {
    assert_eq!(FD_REGEX.captures("@10").unwrap().at(2).unwrap(), "10");
}
