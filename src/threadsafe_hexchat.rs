
//! A thread-safe wrapper for the `Hexchat` object. The methods of the object
//! will execute on the Hexchat main thread when called from another thread.
//! The client code doesn't have to worry about synchronization; that's taken
//! care of internally by `ThreadSafeHexchat`.

use std::sync::Arc;

use crate::hexchat::*;
use crate::thread_facilities::*;
use crate::threadsafe_context::*;
use crate::threadsafe_list_iterator::*;

/// A thread-safe wrapper for the `Hexchat` object.
/// It implements a subset of the methods provided by the wrapped object.
/// A lot of methods don't make sense to expose; those that do should provide
/// enough utility to code that needs to invoke Hexchat from other threads.
///
#[derive(Clone, Copy)]
pub struct ThreadSafeHexchat {
    hc: &'static Hexchat,
}

unsafe impl Send for ThreadSafeHexchat {}
unsafe impl Sync for ThreadSafeHexchat {}

impl ThreadSafeHexchat {
    /// Constructs a `ThreadSafeHexchat` object that wraps `Hexchat`.
    pub (crate) 
    fn new(hc: &'static Hexchat) -> Self {
        ThreadSafeHexchat { hc }
    }
    
    /// Prints the string passed to it to the active Hexchat window.
    /// # Arguments
    /// * `text` - The text to print.
    ///
    pub fn print(&self, text: &str) {
        let text = std::sync::Arc::new(text.to_string());
        let result = main_thread(move |hc| hc.print(&text));
        result.get();
    }
    
    /// Invokes the Hexchat command specified by `command`.
    /// # Arguments
    /// * `command` - The Hexchat command to invoke.
    ///
    pub fn command(&self, command: &str) {
        let command = std::sync::Arc::new(command.to_string());
        let result = main_thread(move |hc| hc.command(&command));
        result.get();
    }
    
    /// Returns a `ThreadSafeContext` object bound to the requested channel.
    /// The object provides methods like `print()` that will execute the 
    /// Hexchat print command in that tab/window related to the context.
    /// The `Context::find()` can also be invoked to find a context.
    /// # Arguments
    /// * `network`  - The network (e.g. "freenode") of the context.
    /// * `channel`  - The channel name for the context (e.g. "##rust").
    /// # Returns
    /// * the thread-safe context was found, i.e. if the user is joined to the
    ///   channel specified currently, a `Some(<Context>)` is returned with the
    ///   context object; `None` otherwise.
    ///
    pub fn find_context(&self, 
                        network : &str, 
                        channel : &str
                       ) -> Option<ThreadSafeContext>
    {
        let data = Arc::new((network.to_string(),
                             channel.to_string()));
        main_thread(move |hc| {
            hc.find_context(&data.0, &data.1).map(ThreadSafeContext::new)
        }).get()
    }

    /// This should be invoked from the main thread. The context object returned
    /// can then be moved to a thread that uses it. Executing this from a
    /// separate thread may not grab the expected context internally.
    ///
    /// Returns a `ThreadSafeContext` object for the current context
    /// currently visible in the app). This object can be used to invoke
    /// the Hexchat API within the context the object is bound to. Also,
    /// `Context::get()` will return a context object for the current context.
    /// # Returns
    /// * The `ThreadSafeContext` for the currently active context. This usually 
    ///   means the channel window the user has visible in the GUI.
    ///
    pub fn get_context(&self) -> Option<ThreadSafeContext> {
        self.hc.get_context().map(ThreadSafeContext::new)
    }
        
    /// Retrieves the info data with the given `id`. It returns None on failure
    /// and `Some(String)` on success. All information is returned as String
    /// data - even the "win_ptr"/"gtkwin_ptr" values, which can be parsed
    /// and cast to pointers.
    /// # Arguments
    /// * `id` - The name/identifier for the information needed. A list of
    ///          the names for some of these can be found on the Hexchat
    ///          Plugin Interface page under `hexchat_get_info()`. These include
    ///          "channel", "network", "topic", etc.
    /// # Returns
    /// * `Some(<String>)` is returned with the string value of the info
    ///   requested. `None` is returned if there is no info with the requested
    ///   `id`.
    ///
    pub fn get_info(&self, id: &str) -> Option<String> {
        let id = Arc::new(id.to_string());
        main_thread(move |hc| {
            hc.get_info(&id)
        }).get()
    }
    
    /// Creates an iterator for the requested Hexchat list. This is modeled
    /// after how Hexchat implements the listing feature: rather than load
    /// all the list items up front, an internal list pointer is advanced
    /// to the current item, and the fields of which are accessible through
    /// the iterator's `.get_field()` function. 
    /// See the Hexchat Plugin Interface web page for more information on the
    /// related lists.
    /// # Arguments
    /// * `name` - The name of the list to iterate over.
    /// # Returns
    /// * If the list exists, `Some(ThreadSafeListIterator)` is returned; `None`
    ///   otherwise.
    ///
    pub fn list_get(&self, list: &str) -> Option<ThreadSafeListIterator> {
        let list = Arc::new(list.to_string());
        main_thread(move |hc| {
            hc.list_get(&list).map(ThreadSafeListIterator::create)
        }).get()
    }
}

