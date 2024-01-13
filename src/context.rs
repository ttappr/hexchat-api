
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

use std::error;
use std::fmt;
use std::ffi::CString;
use std::rc::Rc;

use crate::hexchat::{Hexchat, hexchat_context, HexchatError};
use crate::hexchat_entry_points::HEXCHAT;
use crate::list_iterator::ListIterator;
use crate::utils::*;

use ContextError::*;
use HexchatError::*;

#[derive(Debug)]
struct ContextData {
    hc          : &'static Hexchat,
    network     : CString,
    channel     : CString,
}
/// Any channel in Hexchat has an associated IRC network name and channel name.
/// The network name and channel name are closely associated with the Hexchat
/// concept of contexts. Hexchat contexts can also be thought of as the 
/// tabs, or windows, open in the UI that have the user joined to their various
/// "chat rooms". To access a specific chat window in Hexchat, its context 
/// can be acquired and used. This library's `Context` objects represent the
/// Hexchat contexts and can be used to interact with the specific 
/// channels/windows/tabs that he user has open. For instance if your plugin
/// needs to output only to specific channels, rather than the default window
/// (which is the one currently open) - it can acquire the appropriate context
/// using `Context::find("some-network", "some-channel")`, and use the object
/// returned to invoke a command, `context.command("SAY hello!")`, or print, 
/// `context.print("Hello!")`, or perform other operations.
///
#[derive(Debug, Clone)]
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
                let nwstr = CString::new("network").unwrap();
                let chstr = CString::new("channel").unwrap();
                let network = (hc.c_get_info)(hc, nwstr.as_ptr());
                let channel = (hc.c_get_info)(hc, chstr.as_ptr());
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
    ///
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
    ///
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
    ///
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
    ///
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
    ///
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
    ///
    pub fn list_get(&self, list: &str)
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

/// The `Context` functions may encounter an error when invoked depending on
/// whether the network name and channel name they're bound to are currently
/// valid. 
/// # Variants
/// * `AcquisitionFailed`   - The function was unable to acquire the desired
///                           context associated with its network and channel
///                           names.
/// * `OperationFailed`     - The context acquisition succeeded, but there is
///                           some problem with the action being performed,
///                           for instance the requested list for 
///                           `ctx.get_listiter("foo")` doesn't exist.
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
                write!(f, "AcquisitionFailed(\"{}\", \"{}\")",
                          network, channel)
            },
            OperationFailed(reason) => {
                write!(f, "OperationFailed(\"{}\")", reason)
            },
        }
    }
}
/*
// See the following for the methods to implement:
//  https://doc.rust-lang.org/src/std/error.rs.html#48-153
impl error::Error for ContextError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Option::from(match self {
            AcquisitionFailed(network, channel, err) => {
                match err {
                    Some(err) => { err },
                    None => { None },
                }
            },
            OperationFailed(msg) => { None },
        })
    }
}
*/
