
//! A thread-safe version of `Context`. The methods of these objects will 
//! execute on the main thread of Hexchat. Invoking them is virutally the same 
//! as with `Context` objects.

use std::sync::Arc;
use std::fmt;
use std::sync::RwLock;

use send_wrapper::SendWrapper;

use crate::context::*;
use crate::thread_facilities::*;
use crate::threadsafe_list_iterator::*;


/// A thread-safe version of `Context`. Its methods automatically execute on
/// the Hexchat main thread. The full set of methods of `Context` aren't 
/// fully implemented for this struct because some can't be trusted to produce
/// predictable results from other threads. For instance `.set()` from a thread
/// would only cause Hexchat to momentarily set its context, but Hexchat's
/// context could change again at any moment while the other thread is 
/// executing.
/// 
///
#[derive(Clone, Debug)]
pub struct ThreadSafeContext {
    ctx : Arc<RwLock<Option<SendWrapper<Context>>>>,
}

unsafe impl Send for ThreadSafeContext {}
unsafe impl Sync for ThreadSafeContext {}

impl ThreadSafeContext {
    /// Creates a new `ThreadSafeContext` object, which wraps a `Context` object
    /// internally.
    pub (crate) 
    fn new(ctx: Context) -> Self {
        Self { ctx: Arc::new(RwLock::new(Some(SendWrapper::new(ctx)))) }
    }
    
    /// Gets the current `Context` wrapped in a `ThreadSafeContext` object.
    /// This method should be called from the Hexchat main thread for it
    /// to contain a predictable `Context`. Executing it from a thread can
    /// yield a wrapped `Context` for an unexpected channel.
    ///
    pub fn get() -> Option<Self> {
        main_thread(|_| Context::get().map(Self::new)).get()
    }
    
    /// Gets a `ThreadSafeContext` object associated with the given channel.
    /// # Arguments
    /// * `network` - The network of the channel to get the context for.
    /// * `channel` - The channel to get the context of.
    /// # Returns
    /// * `Some(ThreadSafeContext)` on success, and `None` on failure.
    ///
    pub fn find(network: &str, channel: &str) -> Option<Self> {
        let data = (network.to_string(), channel.to_string());
        main_thread(move |_| {
            Context::find(&data.0, &data.1).map(Self::new)
        }).get()
    }

    /// Prints the message to the `ThreadSafeContext` object's Hexchat context.
    /// This is how messages can be printed to Hexchat windows apart from the
    /// currently active one.    
    pub fn print(&self, message: &str) -> Result<(), ContextError> {
        let message = message.to_string();
        let me = self.clone();
        main_thread(move |_| {
            me.ctx.read().unwrap().as_ref()
                  .expect("Context dropped from threadsafe context.")
                  .print(&message)
        }).get()
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
    pub fn command(&self, command: &str) -> Result<(), ContextError> {
        let command = command.to_string();
        let me = self.clone();
        main_thread(move |_| {
            me.ctx.read().unwrap().as_ref()
                  .expect("Context dropped from threadsafe context.")
                  .command(&command)
        }).get()
    }

    /// Gets information from the channel/window that the `ThreadSafeContext` 
    /// object holds an internal pointer to.
    ///
    pub fn get_info(&self, info: &str) -> Result<Option<String>, ContextError> {
        let info = info.to_string();
        let me = self.clone();
        main_thread(move |_| {
            me.ctx.read().unwrap().as_ref()
                  .expect("Context dropped from threadsafe context.")
                  .get_info(&info)
        }).get()
    }
    
    /// Issues a print event to the context held by the `ThreadSafeContext` 
    /// object.    
    ///
    pub fn emit_print(&self, event_name: &str, var_args: &[&str])
        -> Result<(), ContextError>
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
                  .expect("Context dropped from threadsafe context.")
                  .emit_print(&data.0, var_args.as_slice())
        }).get()
    }
    
    /// Gets a `ListIterator` from the context held by the `Context` object.
    /// If the list doesn't exist, the `OK()` result will contain `None`;
    /// otherwise it will hold the `listIterator` object for the requested
    /// list.
    ///
    pub fn list_get(&self, 
                    name: &str) 
        -> Result<Option<ThreadSafeListIterator>, ContextError>
    {
        let name = name.to_string();
        let me = self.clone();
        main_thread(move |_| {
            if let Some(ctx) = me.ctx.read().unwrap().as_ref() {
                match ctx.list_get(&name) {
                    Ok(opt) => {
                        if let Some(list) = opt {
                            Ok(Some(ThreadSafeListIterator::create(list)))
                        } else {
                            Ok(None)
                        }
                    },
                    Err(err) => Err(err),
                }
            } else {
                Err(ContextError::ContextDropped(
                    "Context dropped from threadsafe context.".to_string()))
            }
        }).get()
    }
}

impl fmt::Display for ThreadSafeContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.ctx)
    }
}

impl Drop for ThreadSafeContext {
    fn drop(&mut self) {
        if Arc::strong_count(&self.ctx) == 1 
            && self.ctx.read().unwrap().is_some() {
            let me = self.clone();
            main_thread(move |_| {
                me.ctx.write().unwrap().take();
            });
        }
    }
}