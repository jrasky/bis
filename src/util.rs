use std::path::{Path, PathBuf};
use std::ffi::{CString};
use std::os::unix::prelude::*;

use std::env;
use std::ptr;
use std::slice;
use std::io;

#[macro_export]
macro_rules! tryp {
    ($e:expr) => ({
        match $e {
            Ok(e) => e,
            Err(e) => panic!("{}", e),
        }
    })
}

#[macro_export]
macro_rules! tryf {
    ($e:expr, $($arg:tt)*) => ({
        match $e {
            Ok(e) => e,
            Err(e) => return Err(format!($($arg)*, err=e))
        }
    })
}

// work around lack of DST
pub fn build_string(ch:char, count:usize) -> String {
    let mut s = String::new();
    let mut i = 0usize;
    loop {
        if i == count {
            return s;
        }
        s.push(ch);
        i += 1;
    }
}

pub fn expand_path(path:PathBuf) -> PathBuf {
    match path.clone().relative_from(Path::new("~")) {
        None => path,
        Some(part) => match env::home_dir() {
            None => PathBuf::from("/"),
            Some(val) => PathBuf::from(val)
        }.join(part)
    }
}

pub fn condense_path(path:PathBuf) -> PathBuf {
    match env::home_dir() {
        None => path,
        Some(homep) => match path.clone().relative_from(homep.as_path()) {
            None => path,
            Some(ref part) => PathBuf::from("~").join(part)
        }
    }
}

pub fn env_to_cstring<T: Iterator<Item=(String, String)>>(vars: T) -> io::Result<Vec<CString>> {
    let mut buf = Vec::with_capacity(match vars.size_hint() {
        (_, Some(len)) => len,
        (len, _) => len
    });
    for item in vars {
        buf.push(try!(format!("{}={}", item.0, item.1).as_os_str().to_cstring()));
    }
    Ok(buf)
}

#[inline]
pub fn bytes_to_i32(arr: &[u8; 4]) -> &i32 {
    unsafe {(arr.as_ptr() as *const i32).as_ref()}.unwrap()
}

#[inline]
pub fn bytes_to_u64(arr: &[u8; 8]) -> &u64 {
    unsafe {(arr.as_ptr() as *const u64).as_ref()}.unwrap()
}

#[inline]
pub fn i32_to_bytes(val: &i32) -> &[u8] {
    unsafe {slice::from_raw_parts(val as *const _ as *const u8, 4)}
}

#[inline]
pub fn u64_to_bytes(val: &u64) -> &[u8] {
    unsafe {slice::from_raw_parts(val as *const _ as *const u8, 8)}
}

#[inline]
pub fn align_len(len: usize, to: usize) -> usize {
    (len + to - 1) & !(to - 1)
}

pub fn find_hole<T:Iterator<Item=usize>>(iter: T) -> usize {
    let mut last = 0;
    for key in iter {
        if key - last > 1 {
            // we've found a hole
            return key - 1;
        } else {
            last = key;
        }
    }
    assert!(last + 1 != 0);
    return last + 1;
}

pub fn read_string<T:io::Read>(reader:&mut T, size:usize) -> io::Result<String> {
    let mut buf = Vec::with_capacity(size);
    // we've already allocated it with the right capacity, so we're ok
    unsafe {buf.set_len(size)};
    match reader.read(buf.as_mut_slice()) {
        Ok(bytes) => Ok(String::from_utf8_lossy(buf[0..bytes].as_slice()).into_owned()),
        Err(e) => Err(e)
    }
}

#[test]
fn build_string_test() {
    assert!(build_string('a', 5) == String::from_str("aaaaa"));
}

#[test]
fn expand_path_test() {
    // tests require the HOME env set
    assert!(expand_path(PathBuf::from("~/Documents/scripts/")) ==
            env::home_dir().unwrap().join("Documents/scripts/"));
    assert!(expand_path(PathBuf::from("/etc/wash/")) == PathBuf::from("/etc/wash/"));
}

#[test]
fn condense_path_test() {
    // tests require the HOME env set
    assert!(condense_path(env::home_dir().unwrap().join("Documents/scripts/")) ==
            PathBuf::from("~/Documents/scripts/"));
    assert!(condense_path(PathBuf::from("/home/")) == PathBuf::from("/home/"));
    assert!(condense_path(PathBuf::from("/etc/wash/")) == PathBuf::from("/etc/wash/"));
}
