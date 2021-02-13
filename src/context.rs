#![allow(dead_code)]

//! Objects of the `Context` class represent Hexchat contexts, which are
//! associated with channels the user is currently in. These are usually
//! associated with an open window/tab in their client GUI. The `Context`
//! objects provide a convenient set of commands that mirror those in the
//! main `Hexchat` class, but when executed they perform their operation in
//! the window/tab/channel that the `Context` is bound to. The network and
//! server strings are used internally to acquire context pointers, which
//! are then used to switch context for a command operation and switch back
//! to the previously active context. On each command a check is performed to
//! ensure the `Context` is still valid. If that fails a `AcquisitionFailed`
//! error is returned with the network/channel strings as data.

use libc::{c_char, c_void};
use std::error;
use std::ffi::{CString, CStr};
use std::cell::RefCell;
use std::rc::Rc;
use crate::hexchat::{Hexchat, hexchat_context};
use crate::hexchat_entry_points::HEXCHAT;
use crate::list_iterator::{ListIterator, ListError, FieldValue};
use crate::utils::*;
use crate::cbuf;
use crate::errors::*;
use std::fmt;

use ContextError::*;
use HexchatError::*;

#[derive(Debug)]
struct ContextData {
    hc          : &'static Hexchat,
    network     : CString,
    channel     : CString,
}
#[derive(Debug)]
pub struct Context {
    data    : Rc<ContextData>,
}

impl Context {
    /// This will create a new `Context` object holding an internal pointer to
    /// the requested network/channel, if it exists. The object will be
    /// returned as a `Some<Context>` if the context is found, or `None` if
    /// not.
    pub fn find(network: &str, channel: &str) -> Option<Self> {
        let csnetwork = str2cstring(network);
        let cschannel = str2cstring(channel);
        let hc = unsafe { &*HEXCHAT };
        let context_ptr;
        unsafe {
            context_ptr = (hc.c_find_context)(hc,
                                              csnetwork.as_ptr(),
                                              cschannel.as_ptr());
        }
        if !context_ptr.is_null() {
            let ctx = Context {
                data: Rc::new(
                    ContextData {
                        hc,
                        network  : csnetwork,
                        channel  : cschannel,
                    })};
            Some(ctx)
        } else {
            None
        }
    }

    /// This will create a new `Context` that represents the currently active
    /// context (window/tab, channel/network) open on the user's screen. A
    /// `Result<Context, ()>` is returned with either the context, or an
    /// error result if it coulnd't be obtained.
    pub fn get() -> Option<Self> {
        unsafe {
            let hc = &*HEXCHAT;
            let ctx_ptr = (hc.c_get_context)(hc);
            if !ctx_ptr.is_null() {
                let network = (hc.c_get_info)(hc, cbuf!("network"));
                let channel = (hc.c_get_info)(hc, cbuf!("channel"));
                let ctx = Context {
                    data: Rc::new(
                        ContextData {
                            hc,
                            network  : pchar2cstring(network),
                            channel  : pchar2cstring(channel),
                        })
                };
                Some(ctx)
            } else{
                None
            }
        }
    }

    pub (crate)
    fn from_pointer(pointer: *const c_void) -> Option<Self> {
        if pointer.is_null() {
            None
        } else {
            unsafe {
                let hc = &(*HEXCHAT);
                let prior = (hc.c_get_context)(hc);
                if !prior.is_null() {
                    if (hc.c_set_context)(hc, pointer) > 0 {
                        let ctx = Context::get();
                        (hc.c_set_context)(hc, prior);
                        ctx
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    /// Private method to try and acquire a context pointer for a `Context`
    /// object. Contexts can go bad in Hexchat: if the user shuts a tab/window
    /// or leaves a channel, using a context associated with that channel
    /// is no longer valid. Or the Hexchat client could disconnect; in which
    /// case, using old context pointers can cause unexpected problems.
    /// So `Context` objects need to reacquire the pointer for each command
    /// invocation. If successful, `Ok(ptr)` is returned with the pointer value;
    /// `AcquisitionFailed(network, channel)` otherwise.
    #[inline]
    fn acquire(&self) -> Result<*const hexchat_context, ContextError> {
        let data = &*self.data;
        let ptr = unsafe {
            (data.hc.c_find_context)(data.hc,
                                     data.network.as_ptr(),
                                     data.channel.as_ptr())
        };
        if !ptr.is_null() {
            Ok(ptr)
        } else {
            Err(AcquisitionFailed(cstring2string(&data.network),
                                  cstring2string(&data.channel)))
        }
    }

    /// Sets the currently active context to the context the `Context` object
    /// points to internally.
    pub fn set(&self) -> Result<(), ContextError> {
        let data = &*self.data;
        unsafe {
            let ptr = self.acquire()?;
            if (data.hc.c_set_context)(data.hc, ptr) > 0 {
                Ok(())
            } else { Err(OperationFailed(".set() failed.".to_string())) }
        }
    }

    /// Prints the message to the `Context` object's Hexchat context. This is
    /// how messages can be printed to Hexchat windows apart from the currently
    /// active one.
    pub fn print(&self, message: &str) -> Result<(), ContextError> {
        let data = &*self.data;
        unsafe {
            let ptr = self.acquire()?;
            let prior = (data.hc.c_get_context)(data.hc);
            (data.hc.c_set_context)(data.hc, ptr);
            data.hc.print(message);
            (data.hc.c_set_context)(data.hc, prior);
            Ok(())
        }
    }

    /// Issues a print event to the context held by the `Context` object.
    pub fn emit_print(&self, event_name: &str, var_args: &[&str])
        -> Result<(), ContextError>
    {
        let data = &*self.data;
        unsafe {
            let ptr = self.acquire()?;
            let prior = (data.hc.c_get_context)(data.hc);
            (data.hc.c_set_context)(data.hc, ptr);
            let result = data.hc.emit_print(event_name, var_args);
            (data.hc.c_set_context)(data.hc, prior);
            if let Err(CommandFailed(msg)) = result {
                Err(OperationFailed(msg))
            } else {
                Ok(())
            }
        }
    }

    /// Issues a command in the context held by the `Context` object.
    pub fn command(&self, command: &str) -> Result<(), ContextError> {
        let data = &*self.data;
        unsafe {
            let ptr = self.acquire()?;
            let prior  = (data.hc.c_get_context)(data.hc);
            (data.hc.c_set_context)(data.hc, ptr);
            data.hc.command(command);
            (data.hc.c_set_context)(data.hc, prior);
            Ok(())
        }
    }

    /// Gets information from the channel/window that the `Context` object
    /// holds an internal pointer to.
    pub fn get_info(&self, list: &str) -> Result<Option<String>, ContextError>
    {
        let data = &*self.data;
        unsafe {
            let ptr = self.acquire()?;
            let prior = (data.hc.c_get_context)(data.hc);
            (data.hc.c_set_context)(data.hc, ptr);
            let result = data.hc.get_info(list);
            (data.hc.c_set_context)(data.hc, prior);
            Ok(result)
        }
    }

    /// Gets a `ListIterator` from the context held by the `Context` object.
    /// If the list doesn't exist, the `OK()` result will contain `None`;
    /// otherwise it will hold the `listIterator` object for the requested
    /// list.
    pub fn get_listiter(&self, list: &str)
        -> Result<Option<ListIterator>, ContextError>
    {
        let data = &*self.data;
        unsafe {
            let ptr = self.acquire()?;
            let prior = (data.hc.c_get_context)(data.hc);
            (data.hc.c_set_context)(data.hc, ptr);
            let iter = ListIterator::new(list);
            (data.hc.c_set_context)(data.hc, prior);
            Ok(iter)
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let data    = &*self.data;
        let network = cstring2string(&data.network);
        let channel = cstring2string(&data.channel);

        write!(f, "Context(\"{}\", \"{}\")", network, channel)
    }
}

#[derive(Debug, Clone)]
pub enum ContextError {
    AcquisitionFailed(String, String),
    OperationFailed(String),
}


impl error::Error for ContextError {}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AcquisitionFailed(network, channel) => {
                write!(f, "An existing `Context` for {}/{} has failed to \
                           acquire a valid Hexchat context pointer while \
                           performing an operation. Contexts can go bad \
                           if the client disconnects, the user shuts the \
                           associated tab/window, or they part/leave the \
                           related channel.",
                          network, channel)
            },
            OperationFailed(reason) => {
                write!(f, "The `Context` is still valid, but an operation \
                           didn't succeed with the given reason: {}", reason)
            },
        }
    }
}

/*
impl error::Error for ContextError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.side)
    }
}
*/


