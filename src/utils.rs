
#![allow(unused_macros, dead_code)]

use libc::c_char;
use std::ffi::{CString, CStr};
#[allow(unused_imports)]
use std::sync::Arc;

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
/// Used internally by the Rust Hexchat API.
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
/// `outp!(hc, "<format_string>", arg1, arg2, ...)`. To print from another 
/// thread `outpth!()` can be used.
/// ```
/// outp!(fmt, argv, ...);
/// outp!(arg);
/// outp!(ctx=(network, channel), argv, ...);
/// outp!(ctx=(network, channel), arg);
/// ```
/// # Arguments
/// * `ctx=(network, channel)` - Sets the context to print in.
/// * `fmt`     - The format string.
/// * `argv`    - The varibale length formatted arguments.
/// 
#[macro_export]
macro_rules! outp {
    ( ctx = ($network:expr, $channel:expr), $fmt:expr, $( $argv:expr ),+ ) => {
        #[allow(unused_must_use)]
        if let Some(orig_ctx) = HEXCHAT.get_context() {
            if let Some(ctx) = hexchat_api::Context::find(&$network, &$channel) 
            {
                ctx.set();
                HEXCHAT.print(&format!($fmt, $($argv),+));
                orig_ctx.set();
            } else {
                panic!("Can't find context for ({}, {})", &$network, &$channel);
            }
        } else {
            panic!("Unable to acquire local context.");
        }
    };
    ( ctx = ($network:expr, $channel:expr), $arg:expr ) => {
        #[allow(unused_must_use)]
        if let Some(orig_ctx) = HEXCHAT.get_context() {
            if let Some(ctx) = hexchat_api::Context::find(&$network, &$channel) 
            {
                ctx.set();
                HEXCHAT.print( $arg );
                orig_ctx.set();
            } else {
                panic!("Can't find context for ({}, {})", &$network, &$channel);
            }
        } else {
            panic!("Unable to acquire local context.");
        }
    };
    ( $fmt:expr, $( $argv:expr ),+ ) => {
        HEXCHAT.print(&format!($fmt, $($argv),+))
    };
    ( $arg:expr ) => {
        HEXCHAT.print( $arg )
    };
}

/// Similar to `outp!()`, that can be used from spawned threads to print to
/// the active Hexchat window. This should not be invoked from the Hexchat
/// main thread.
/// ```
/// outpth!(fmt, argv, ...);
/// outpth!(arg);
/// outpth!(ctx=(network, channel), argv, ...);
/// outpth!(ctx=(network, channel), arg);
/// ```
/// # Arguments
/// * `ctx=(network, channel)` - Sets the context to print in.
/// * `fmt`     - The format string.
/// * `argv`    - The varibale length formatted arguments.
/// 
#[macro_export]
macro_rules! outpth {
    ( ctx = ($network:expr, $channel:expr), $arg:expr ) => {{
        // TODO - Make a tuple for these string values instead of a separate
        //        Arc for each one.
        let data = std::sync::Arc::new(($arg.to_string(),
                                        $network.to_string(),
                                        $channel.to_string()));
        #[allow(unused_must_use)]
        hexchat_api::main_thread(move |HEXCHAT| {
            if let Some(orig_ctx) = HEXCHAT.get_context() {
                if let Some(ctx) = hexchat_api::Context::find(&data.1, &data.2) 
                {
                    ctx.set();
                    HEXCHAT.print(&data.0);
                    orig_ctx.set();
                } else {
                    panic!("Can't find context for ({}, {})", &data.1, &data.2);
                }
            } else {
                panic!("Unable to acquire local context.");
            }
        });
    }};
    (  ctx = ($network:expr, $channel:expr), $fmt:expr, $( $argv:expr ),+ )=> {{
        let fm_msg = format!($fmt, $($argv),+);
        let data   = std::sync::Arc::new(($fm_msg.to_string(),
                                          $network.to_string(),
                                          $channel.to_string()));
        #[allow(unused_must_use)]
        hexchat_api::main_thread(move |hc| {
            if let Some(orig_ctx) = hc.get_context() {
                if let Some(ctx) = hexchat_api::Context::find(&data.1, &data.2) 
                { 
                    ctx.set();
                    hc.print(&data.0);
                    orig_ctx.set();
                } else {
                    panic!("Can't find context for ({}, {})", &data.1, &data.2);
                }
            } else {
                panic!("Unable to acquire local context.");
            }
        });
    }};
    ( $arg:expr ) => {{
        let rc_msg = std::sync::Arc::new($arg.to_string());
        hexchat_api::main_thread(move |hc| hc.print(&rc_msg));
    }};
    ( $fmt:expr, $( $argv:expr ),+ ) => {{
        let fm_msg = format!($fmt, $($argv),+);
        let rc_msg = std::sync::Arc::new(fm_msg.to_string());
        hexchat_api::main_thread(move |hc| hc.print(&rc_msg));
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
