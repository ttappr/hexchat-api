
#![allow(unused_macros, dead_code)]

use libc::c_char;
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

/// Reduces the syntax required to output formatted text to the current
/// hexchat window. Internally it invokes 
/// `hexchat.print(&format!("<format-string>", arg1, arg2, ...)`.
/// Using the macro, this becomes 
/// `outp!(hc, "<format_string>, arg1, arg2, ...)`. To print from another thread
/// `outpth!()` can be used.
/// ```
/// outp!(obj, fmt, argv, ...)
/// ```
/// # Arguments
/// * `obj`     - The Hexchat struct reference.
/// * `fmt`     - The format string.
/// * `argv`    - The varibale length formatted arguments.
/// 
#[macro_export]
macro_rules! outp {
    ( $obj:ident, $fmt:expr, $( $argv:expr ),+ ) => {
        $obj.print(&format!($fmt, $($argv),+))
    };
    ( $obj:ident, $( $argv:expr ),+ ) => {
        $obj.print( $($argv),+ )
    };
}

/// Similar to `outp!()`, that can be used from spawned threads to print to
/// the active Hexchat window. This should not be invoked from the Hexchat
/// main thread.
/// ```
/// outpth!(obj, fmt, argv, ...)
/// ```
/// # Arguments
/// * `obj`     - The Hexchat struct reference.
/// * `fmt`     - The format string.
/// * `argv`    - The varibale length formatted arguments.
/// 
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
/// * `start`   - Which offset in the pchar array to start at.
///
/// # Returns
/// A String Vec mirroring the data passed to it via `pchar`.
///
pub (crate)
fn argv2svec(pchar: *const *const c_char, start: usize) -> Vec<String>
{
    // From the Hexchat document site at
    // (https://hexchat.readthedocs.io/en/latest/plugins.html), the first string
    // of word and word_eol shouldn't be read:
    //     These arrays are simply provided for your convenience. You are not
    //     allowed to alter them. Both arrays are limited to 32 elements
    //     (index 31). word[0] and word_eol[0] are reserved and should not be
    //     read.
    unsafe {
        let mut svec = vec![];
        let mut i    = start;
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
/// Some strings coming from Hexchat my contain invalid characters. This
/// function guards against them offecting the system by replacing those
/// characters with a default character.
pub (crate)
fn cstring2string(cstring: &CString) -> String {
    cstring.to_string_lossy().into_owned()
}
