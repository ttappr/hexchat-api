
#![allow(unused_macros, dead_code)]

use libc::c_char;
use std::ffi::{CString, CStr};
#[allow(unused_imports)]
use std::sync::Arc;

use crate::{Context, Hexchat, PHEXCHAT};

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

/// Reduces the syntax required to output formatted text to the current
/// hexchat window. Internally it invokes 
/// `hexchat.print(&format!("<format-string>", arg1, arg2, ...)`.
/// Using the macro, this becomes 
/// `hc_print!(hc, "<format_string>", arg1, arg2, ...)`. To print from another 
/// thread `hc_print_th!()` can be used.
/// ```
/// use hexchat_api::hc_print;
/// hc_print!(fmt, argv, ...);
/// hc_print!(arg);
/// hc_print!(ctx=(network, channel), argv, ...);
/// hc_print!(ctx=(network, channel), arg);
/// ```
/// # Arguments
/// * `ctx=(network, channel)` - Sets the context to print in.
/// * `fmt`     - The format string.
/// * `argv`    - The varibale length formatted arguments.
/// 
#[macro_export]
macro_rules! hc_print {
    ( ctx = ($network:expr, $channel:expr), $fmt:literal, $( $argv:expr ),+ ) 
    => {
        hexchat_api::
        print_with_ctx_inner(&$network, &$channel, &format!($fmt, $($argv),+));
    };
    ( ctx = ($network:expr, $channel:expr), $arg:expr ) => {
        hexchat_api::print_with_ctx_inner(&$network, &$channel, $arg);
    };
    ( $fmt:literal, $( $argv:expr ),+ ) => {
        hexchat_api::print_inner(&format!($fmt, $($argv),+))
    };
    ( $arg:literal ) => {
        hexchat_api::print_inner( $arg )
    };
}

/// Used by `hc_print!()` to print to a specific context. This function is
/// not intended to be used directly.
/// 
pub fn print_with_ctx_inner(network: &str, channel: &str, msg: &str) {
    let hc = unsafe { &*PHEXCHAT as &Hexchat };
    if let Some(orig_ctx) = hc.get_context() {
        if let Some(ctx) = Context::find(network, channel) {
            let _ = ctx.set();
            hc.print(msg);
            let _ = orig_ctx.set();
        } else {
            panic!("Can't find context for ({}, {})", network, channel);
        }
    } else {
        panic!("Unable to acquire local context.");
    }
}

/// Used by `hc_print!()` to print to the active Hexchat window. This function
/// is not intended to be used directly.
/// 
pub fn print_inner(msg: &str) {
    let hc = unsafe { &*PHEXCHAT as &Hexchat };
    hc.print(msg);
}

/// Similar to `hc_print!()`, that can be used from spawned threads to print to
/// the active Hexchat window. This should not be invoked from the Hexchat
/// main thread.
/// ```
/// use hexchat_api::hc_print_th;
/// hc_print_th!(fmt, argv, ...);
/// hc_print_th!(arg);
/// hc_print_th!(ctx=(network, channel), argv, ...);
/// hc_print_th!(ctx=(network, channel), arg);
/// ```
/// # Arguments
/// * `ctx=(network, channel)` - Sets the context to print in.
/// * `fmt`     - The format string.
/// * `argv`    - The varibale length formatted arguments.
/// 
#[macro_export]
macro_rules! hc_print_th {
    ( ctx = ($network:expr, $channel:expr), $arg:expr ) => {{
        // TODO - Make a tuple for these string values instead of a separate
        //        Arc for each one.
        let data = std::sync::Arc::new(($arg.to_string(),
                                        $network.to_string(),
                                        $channel.to_string()));
        hexchat_api::main_thread(move |_| {
            hexchat_api::print_with_ctx_inner(&data.1, &data.2, &data.0);
        });
    }};
    (  ctx = ($network:expr, $channel:expr), $fmt:literal, $( $argv:expr ),+ )
    => {{
        let fm_msg = format!($fmt, $($argv),+);
        let data   = std::sync::Arc::new(($fm_msg.to_string(),
                                          $network.to_string(),
                                          $channel.to_string()));
        hexchat_api::main_thread(move |_| {
            hexchat_api::print_with_ctx_inner(&data.1, &data.2, &data.0);
        });
    }};
    ( $arg:expr ) => {{
        let rc_msg = std::sync::Arc::new($arg.to_string());
        hexchat_api::main_thread(move |_| hexchat_api::print_inner(&rc_msg));
    }};
    ( $fmt:literal, $( $argv:expr ),+ ) => {{
        let fm_msg = format!($fmt, $($argv),+);
        let rc_msg = std::sync::Arc::new(fm_msg.to_string());
        hexchat_api::main_thread(move |_| hexchat_api::print_inner(&rc_msg));
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
