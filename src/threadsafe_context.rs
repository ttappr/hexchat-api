#![cfg(feature = "threadsafe")]

//! A thread-safe version of `Context`. The methods of these objects will
//! execute on the main thread of Hexchat. Invoking them is virutally the same
//! as with `Context` objects.

use std::sync::Arc;
use std::fmt;
use std::sync::RwLock;

use send_wrapper::SendWrapper;

use crate::HexchatError;
use crate::context::*;
use crate::thread_facilities::*;
use crate::threadsafe_list_iterator::*;

use HexchatError::*;

/// A thread-safe version of `Context`. Its methods automatically execute on
/// the Hexchat main thread. The full set of methods of `Context` aren't
/// fully implemented for this struct because some can't be trusted to produce
/// predictable results from other threads. For instance `.set()` from a thread
/// would only cause Hexchat to momentarily set its context, but Hexchat's
/// context could change again at any moment while the other thread is
/// executing.
///
///
#[derive(Clone)]
pub struct ThreadSafeContext {
    ctx : Arc<RwLock<Option<SendWrapper<Context>>>>,
}

unsafe impl Send for ThreadSafeContext {}
unsafe impl Sync for ThreadSafeContext {}

impl ThreadSafeContext {
    /// Creates a new `ThreadSafeContext` object, which wraps a `Context` object
    /// internally. Only to be called from the main thread internally.
    /// 
    pub (crate)
    fn new(ctx: Context) -> Self {
        Self { ctx: Arc::new(RwLock::new(Some(SendWrapper::new(ctx)))) }
    }

    /// Gets the user's current `Context` wrapped in a `ThreadSafeContext` 
    /// object.
    ///
    pub fn get() -> Result<Self, HexchatError> {
        main_thread(|_| Context::get().map(Self::new)).get()
        .and_then(|r| r.ok_or_else(|| ContextAcquisitionFailed("?, ?".into())))
    }

    /// Gets a `ThreadSafeContext` object associated with the given channel.
    /// # Arguments
    /// * `network` - The network of the channel to get the context for.
    /// * `channel` - The channel to get the context of.
    /// # Returns
    /// * `Ok(ThreadSafeContext)` on success, and `HexchatError` on failure.
    ///
    pub fn find(network: &str, channel: &str) -> Result<Self, HexchatError> {
        let data = (network.to_string(), channel.to_string());
        main_thread(move |_| Context::find(&data.0, &data.1).map(Self::new))
        .get()
        .and_then(|r| r.ok_or_else(|| {
                    let msg = format!("{}, {}", network, channel);
                    ContextAcquisitionFailed(msg) 
                }))
    }

    /// Prints the message to the `ThreadSafeContext` object's Hexchat context.
    /// This is how messages can be printed to Hexchat windows apart from the
    /// currently active one.
    ///
    pub fn print(&self, message: &str) -> Result<(), HexchatError> {
        let message = message.to_string();
        let me = self.clone();
        main_thread(move |_| {
            me.ctx.read().unwrap().as_ref()
                  .ok_or_else(|| ContextDropped("Context dropped from \
                                                 threadsafe context.".into()))?
                  .print(&message)
        }).get().and_then(|r| r)
    }

    /// Prints without waiting for asynchronous completion. This will print
    /// faster than `.print()` because it just stacks up print requests in the
    /// timer queue and moves on without blocking. The downside is errors
    /// will not be checked. Error messages will, however, still be printed
    /// if any occur.
    ///
    pub fn aprint(&self, message: &str) {
        let message = message.to_string();
        let me = self.clone();
        main_thread(move |hc| {
            if let Err(err)
                = me.ctx.read().unwrap().as_ref().unwrap().print(&message) {
                hc.print(
                    &format!("\x0313Context.aprint() failed to acquire \
                              context: {}", err));
                hc.print(
                    &format!("\x0313{}", message));
            }
        });
    }

    /// Issues a command in the context held by the `ThreadSafeContext` object.
    ///
    pub fn command(&self, command: &str) -> Result<(), HexchatError> {
        let command = command.to_string();
        let me = self.clone();
        main_thread(move |_| {
            me.ctx.read().unwrap().as_ref()
                  .ok_or_else(|| ContextDropped("Context dropped from \
                                                 threadsafe context.".into()))?
                  .command(&command)
        }).get().and_then(|r| r)
    }

    /// Gets information from the channel/window that the `ThreadSafeContext`
    /// object holds an internal pointer to.
    ///
    pub fn get_info(&self, info: &str) -> Result<String, HexchatError> {
        let info = info.to_string();
        let me = self.clone();
        main_thread(move |_| {
            me.ctx.read().unwrap().as_ref()
                  .ok_or_else(||ContextDropped("Context dropped from \
                                                threadsafe context.".into()))?
                  .get_info(&info)
            }).get().and_then(|r| r)
    }

    /// Issues a print event to the context held by the `ThreadSafeContext`
    /// object.
    ///
    pub fn emit_print(&self, event_name: &str, var_args: &[&str])
        -> Result<(), HexchatError>
    {
        let var_args: Vec<String> = var_args.iter()
                                            .map(|s| s.to_string())
                                            .collect();
        let data = (event_name.to_string(), var_args);
        let me = self.clone();
        main_thread(move |_| {
            let var_args: Vec<&str> = data.1.iter()
                                            .map(|s| s.as_str())
                                            .collect();
            me.ctx.read().unwrap().as_ref()
                  .ok_or_else(|| ContextDropped("Context dropped from \
                                                 threadsafe context.".into()))?
                  .emit_print(&data.0, var_args.as_slice())
        }).get().and_then(|r| r)
    }

    /// Gets a `ThreadSafeListIterator` from the context.  If the list doesn't 
    /// exist, or a problem occurs, an error will be returned.
    ///
    pub fn list_get(&self,
                    name: &str)
        -> Result<ThreadSafeListIterator, HexchatError>
    {
        let name = name.to_string();
        let me = self.clone();
        main_thread(move |_| {
            if let Some(ctx) = me.ctx.read().unwrap().as_ref() {
                match ctx.list_get(&name) {
                    Ok(list) => {
                        Ok(ThreadSafeListIterator::create(list))
                    },
                    Err(err) => Err(err),
                }
            } else {
                Err(ContextDropped("Context dropped from threadsafe \
                                    context.".to_string()))
            }
        }).get().and_then(|r| r)
    }
    /// Returns the network name associated with the context.
    /// 
    pub fn network(&self) -> Result<String, HexchatError> {
        let me = self.clone();
        main_thread(move |_| {
            if let Some(ctx) = me.ctx.read().unwrap().as_ref() {
                Ok(ctx.network())
            } else {
                Err(ContextDropped("Context dropped from threadsafe \
                                    context.".to_string()))
            }
        }).get().and_then(|r| r)
    }

    /// Returns the channel name associated with the context.
    /// 
    pub fn channel(&self) -> Result<String, HexchatError> {
        let me = self.clone();
        main_thread(move |_| {
            if let Some(ctx) = me.ctx.read().unwrap().as_ref() {
                Ok(ctx.channel())
            } else {
                Err(ContextDropped("Context dropped from threadsafe \
                                    context.".to_string()))
            }
        }).get().and_then(|r| r)
    }
}

impl fmt::Display for ThreadSafeContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.ctx)
    }
}

impl Drop for ThreadSafeContext {
    fn drop(&mut self) {
        if Arc::strong_count(&self.ctx) <= 1
            && self.ctx.read().unwrap().is_some() {
            let me = self.clone();
            main_thread(move |_| {
                me.ctx.write().unwrap().take();
            });
        }
    }
}

impl fmt::Debug for ThreadSafeContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // A bit overkill, but fixes problems with users trying to debug print
        // the object from other threads.
        let me = self.clone();
        let s = main_thread(move |_| {
            if let Ok(guard) = me.ctx.read() {
                if let Some(ctx) =  guard.as_ref() {
                    format!("Context({:?}, {:?})", ctx.network(), ctx.channel())
                } else {
                    "Context(Error getting info)".to_string()
                }
            } else {
                "Context(Error getting info)".to_string()
            }
        }).get().unwrap();
        write!(f, "{}", s)
    }
}