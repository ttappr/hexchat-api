
#![allow(unused_macros, dead_code)]

use libc::{c_char, c_void};
use std::ffi::{CString, CStr};


/// ```&str -> CString``` (provides a C compatible character buffer)
///
/// Wrapper function that creates a `CString` from a `&str`. This is a
/// convenient format for struct fields where a persistent character buffer
/// is needed.
#[inline]
pub (crate)
fn str2cstring(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// ```&str -> *const c_char```
///
/// Creates a C compatible character buffer from the `&str` that can be passed
/// as a parameter to a C call that takes a char pointer:
/// `some_fn(cbuf!("hello"));`. The returned buffer is short-lived; don't
/// assign its return value to a variable - only use this macro function
/// parameters in place.
#[macro_export]
macro_rules! cbuf {
    ( $s:expr ) => { CString::new( $s ).unwrap().as_ptr() };
}

#[macro_export]
macro_rules! fmt {
    ( $obj:ident . printf ( $fmt:expr, $( $argv:expr ),+ ) ) => {
        $obj.print(&format!($fmt, $($argv),+))
    }
}

#[macro_export]
macro_rules! outp {
    ( $obj:ident, $fmt:expr, $( $argv:expr ),+ ) => {
        $obj.print(&format!($fmt, $($argv),+))
    };
    ( $obj:ident, $( $argv:expr ),+ ) => {
        $obj.print( $($argv),+ )
    };
}

// Don't call this macro from the main thread.
#[macro_export]
macro_rules! outpth {
    ( $obj:ident, $fmt:expr, $( $argv:expr ),+ ) => {{
        let fm_msg = format!($fmt, $($argv),+);
        let rc_msg = Arc::new(fm_msg.to_string());
        main_thread(move |$obj| $obj.print(&rc_msg));
    }};
    ( $obj:ident, $argv:expr ) => {{
        let rc_msg = Arc::new(msg.to_string());
        main_thread(move |$obj| $obj.print(&rc_msg));
    }};
}

/// ```*const c_char -> CString```
///
/// Creates an owned `CString` from the character buffer. Useful for saving
/// a string returned by a C call.
#[inline]
pub (crate)
fn pchar2cstring(p_char: *const c_char) -> CString {
    unsafe { CStr::from_ptr(p_char).to_owned() }
}

/// ```*const c_char -> String```
///
/// Creates a new `String` from a character array - typically coming from
/// Hexchat. The conversion is lossy - which means any invalid utf8 chars
/// in the string from Hexchat will be replaced with a default character.
pub (crate)
fn pchar2string(p_char: *const c_char) -> String {
    if p_char.is_null() {
        String::new()
    } else {
        unsafe {
            CStr::from_ptr(p_char).to_string_lossy().into_owned()
        }
    }
}

/// ```*const *const c_char -> Vec<String>```
///
/// Converts a C style character pointer vector (argv) into a Rust String Vec.
/// This function is used to marshal the string parameters Hexchat passes to
/// registered command callbacks.
///
/// NOTE: This function discards the first item in the vector. This is due to
///       how the Hexchat callbacks are passed string data. This function is
///       probably only useful for these specific callbacks.
///
/// # Arguments
/// * `pchar`   - The C array of character pointers.
///
/// # Returns
/// A String Vec mirroring the data passed to it via `pchar`.
///
pub (crate)
fn argv2svec(pchar: *const *const c_char) -> Vec<String>
{
    unsafe {
        let mut svec = vec![];
        let mut i    = 1;
        let mut pval = *pchar.add(i);
        loop {
            // to_string_lossy() protects against invalid chars.
            let s = CStr::from_ptr(pval)
                            .to_string_lossy()
                            .into_owned();
            if (*pchar.add(i)).is_null() || s.is_empty() {
                break;
            }
            i += 1;
            pval = *pchar.add(i);
            svec.push(s);
        }
        svec
    }
}

/// ```&CString -> String``` creates a new String from a CString.
pub (crate)
fn cstring2string(cstring: &CString) -> String {
    cstring.to_string_lossy().into_owned()
}