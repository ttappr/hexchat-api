
//! This file contains the C-facing functions that are registered directly
//! with Hexchat when a client plugin registers a Rust-facing callback.
//! The callbacks in this file wrap the Rust-facing callbacks, marshal
//! the parameters (word, word_eol, etc) for the Rust callbacks.

use libc::{c_int, c_char, c_void};
use std::panic::catch_unwind;

use crate::callback_data::CallbackData;
use crate::hexchat::Eat;
use crate::hexchat::EventAttrs;
use crate::hexchat_entry_points::PHEXCHAT;
use crate::utils::*;

/// An actual callback registered with Hexchat, which proxies for client plugin
/// callbacks. It builds the `String` vectors passed to client callbacks.
/// See [Hexchat API](https://hexchat.readthedocs.io/en/latest/plugins.html)
pub (crate)
extern "C" fn c_callback(word        : *const *const c_char,
                         word_eol    : *const *const c_char,
                         user_data   : *mut c_void)
    -> c_int
{
    catch_unwind(|| {
        let word     = argv2svec(word, 1);
        let word_eol = argv2svec(word_eol, 1);

        unsafe {
            let cd = user_data as *mut CallbackData;
            let hc = &*PHEXCHAT;
            (*cd).command_cb(hc, &word, &word_eol, (*cd).get_user_data()) as _
        }
    }).unwrap_or(Eat::None as i32)
}

/// An actual callback registered with Hexchat, which proxies for client plugin
/// callbacks. It builds the `String` vector and invokes the client plugin's
/// callbacks. The client plugin callback and data is placed within the
/// `user_data`.
pub (crate)
extern "C" fn c_print_callback(word      : *const *const c_char,
                               user_data : *mut c_void)
    -> c_int
{
    catch_unwind(|| {
        let word = argv2svec(word, 1);

        unsafe {
            let cd = user_data as *mut CallbackData;
            let hc = &*PHEXCHAT;
            (*cd).print_cb(hc, &word, (*cd).get_user_data()) as _
        }
    }).unwrap_or(Eat::None as i32)
}

/// An actual callback registered with Hexchat, which proxies for client plugin
/// callbacks.
/// See [Hexchat API](https://hexchat.readthedocs.io/en/latest/plugins.html)
pub (crate)
extern "C" fn c_print_attrs_callback(word      : *const *const c_char,
                                     attrs     : *const EventAttrs,
                                     user_data : *mut c_void)
    -> c_int
{
    catch_unwind(|| {
        let word = argv2svec(word, 1);

        unsafe {
            let cd = user_data as *mut CallbackData;
            let hc = &*PHEXCHAT;
            (*cd).print_attrs_cb(hc, &word, &*attrs, (*cd).get_user_data()) as _
        }
    }).unwrap_or(Eat::None as i32)
}


/// An actual callback registered with Hexchat, which proxies for client plugin
/// callbacks.
pub (crate)
extern "C" fn c_timer_callback(user_data: *mut c_void) -> c_int {
    catch_unwind(|| {
        unsafe {
            let cd = user_data as *mut CallbackData;
            let hc = &*PHEXCHAT;
            (*cd).timer_cb(hc, (*cd).get_user_data())
        }
    }).unwrap_or(0)
}

/// A special case callback. This is used by the multi threading support to
/// put code on the main thread from code running on an independent thread.
/// The `CallbackData` object will ensure that this callback gets unhooked
/// after a one-time callback is executed.
#[allow(dead_code)]
pub (crate)
extern "C" fn c_timer_callback_once(user_data: *mut c_void) -> c_int {
    catch_unwind(|| {
        unsafe {
            let cd = user_data as *mut CallbackData;
            let hc = &*PHEXCHAT;
            (*cd).timer_once_cb(hc, (*cd).get_user_data())
        }
    }).unwrap_or(0)
}

/// An actual callback registered with Hexchat, which proxies for client plugin
/// callbacks.
pub (crate)
extern "C" fn c_fd_callback(fd: c_int, flags: c_int, user_data: *mut c_void)
    -> c_int
{
    catch_unwind(|| {
        unsafe {
            let cd = user_data as *mut CallbackData;
            let hc = &*PHEXCHAT;
            (*cd).fd_cb(hc, fd, flags, (*cd).get_user_data()) as i32
        }
    }).unwrap_or(Eat::None as i32)
}
