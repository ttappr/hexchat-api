
//! This file holds the Hexchat struct used to interface with Hexchat. Its
//! fields are the actual callbacks provided by Hexchat. When Hexchat
//! loads this library, the Hexchat pointer is stored and used by casting it
//! to the struct contained in this file. These native function pointers are
//! private to this crate, and a more Rust-friendly API is provided through
//! this Hexchat interface. 

use libc::{c_int, c_char, c_void, time_t};
use std::{error, ptr};
use std::ffi::CString;
use std::fmt;
use std::fmt::Debug;
use std::ops::FnMut;
use std::ptr::null;
use std::str;

use crate::callback_data::{CallbackData, TimerCallbackOnce};
use crate::context::Context;
use crate::context::ContextError;
use crate::hexchat_callbacks::*;
use crate::hexchat_entry_points::HEXCHAT;
use crate::hook::Hook;
use crate::list_iterator::ListIterator;
use crate::plugin::Plugin;
use crate::user_data::*;
use crate::utils::*;
use crate::threadsafe_hexchat::*;

use crate::cbuf;

/// Value used in example from the Hexchat Plugin Interface doc web page.
const MAX_PREF_VALUE_SIZE: usize =  512;

/// Value specified on the [Hexchat Plugin Interface web page]
/// (https://hexchat.readthedocs.io/en/latest/plugins.html).
const MAX_PREF_LIST_SIZE : usize = 4096; 

// hexchat_send_modes, hexchat_event_attrs_free, pluginpref_delete, 

/// The priorty for a given callback invoked by Hexchat.
pub enum Priority {
    Highest     =  127,
    High        =   64,
    Norm        =    0,
    Low         =  -64,
    Lowest      = -128,
}

/// The return value for client plugin callbacks.
pub enum Eat {
    None        =    0,
    Hexchat     =    1,
    Plugin      =    2,
    All         =    3,
}

/// File descriptor types.
pub enum FD {
    Read        =    1,
    Write       =    2,
    Exception   =    4,
    NotSocket   =    8,
}

/// Used by the `hexthat.strip()` function to determine what to strip from the
/// target string.
pub enum StripFlags {
    StripMIrcColors     = 1,
    StripTextAttributes = 2,
    StripBoth           = 3,
}

/// This is the rust-facing Hexchat API. Each method has a corresponding
/// C function pointer which they wrap and marshal data/from.
/// 
impl Hexchat {
    
    pub fn threadsafe(&self) -> ThreadSafeHexchat {
        ThreadSafeHexchat::new(unsafe { &*HEXCHAT })
    }

    /// Prints the string passed to it to the active Hexchat window.
    /// # Arguments
    /// * `text` - The text to print.
    ///
    pub fn print(&self, text: &str) {
        unsafe { (self.c_print)(self, cbuf!(text)); }
    }

    /// Invokes the Hexchat command specified by `command`.
    /// # Arguments
    /// * `command` - The Hexchat command to invoke.
    ///
    pub fn command(&self, command: &str) {
        unsafe { (self.c_command)(self, cbuf!(command)); }
    }

    /// Registeres a command callback with Hexchat. This will add a user
    /// invocable slash "/" command that can be seen when listing `/help`.
    /// The callback can be a static function, or a closure, that has the form:
    /// ```
    ///     FnMut(&Hexchat, &[String], &[String], &mut UserData)
    ///     -> Eat
    /// ```
    /// Note that the callback parameters include a reference to the `Hexchat`
    /// object as a convenience. This differs from the C interface which doesn't
    /// include it.
    ///
    /// # Arguments
    /// * `name`        - The name of the event that invokes the callback.
    /// * `pri`         - The priority of the callback.
    /// * `callback`    - The static function or closure to register.
    /// * `help`        - Help text displayed by Hexchat for the command.
    /// * `user_data`   - Data passed back to the callback when invoked.
    ///
    /// # Returns
    /// A `Hook` object associated with the callback.
    ///
    pub fn hook_command<F: 'static>(&self,
                                    name        : &str,
                                    pri         : Priority,
                                    callback    : F,
                                    help        : &str,
                                    user_data   : UserData
                                   ) -> Hook
    where 
        F: FnMut(&Hexchat, &[String], &[String], &mut UserData)
           -> Eat
    {
        let hook = Hook::new();
        let ud   = Box::new(
                    CallbackData::new_command_data(
                                      Box::new(callback),
                                      user_data,
                                      hook.clone()
                                  ));
                                  
        let ud   = Box::into_raw(ud) as *mut c_void;
        
        hook.set_cbd(ud);
        
        let help = if !help.is_empty() {
            help
        } else {
            "No help available for this command."
        };
        unsafe {
            // TODO - Consider making an empty help string cause a NULL to be
            //        used as hook_command()'s 5th argument.
            hook.set((self.c_hook_command)(self,
                                           cbuf!(name),
                                           pri as i32,
                                           c_callback,
                                           cbuf!(help),
                                           ud));
        }
        hook
    }

    /// Registers a callback to be called when a certain server event occurs.
    /// For any of these functions, more information can be found at
    /// [Hexchat Plugin Interface](https://hexchat.readthedocs.io/en/latest/plugins.html)
    /// The callback needs to be compatible with this signature:
    ///  ```
    ///  FnMut(&Hexchat, &[String], &[String], &mut UserData)
    ///  -> Eat
    ///  ```
    /// # Arguments
    /// * `name`        - The name of the event to listen for.
    /// * `pri`         - The priority of the callback.
    /// * `callback`    - The callback to invoke when the event occurs.
    /// * `user_data`   - The user data that gets passed back to the callback
    ///                   when it's invoked.
    /// # Returns
    /// * A `Hook` object that can be used to deregister the callback. It
    ///   doesn't need to be retained if not needed.
    ///
    pub fn hook_server<F: 'static>(&self,
                                   name        : &str,
                                   pri         : Priority,
                                   callback    : F,
                                   user_data   : UserData
                                  ) -> Hook
    where 
        F: FnMut(&Hexchat, &[String], &[String], &mut UserData)
           -> Eat
    {
        let hook = Hook::new();
        let ud   = Box::new(
                    CallbackData::new_command_data( 
                                      Box::new(callback),
                                      user_data,
                                      hook.clone()
                                  ));
        let ud = Box::into_raw(ud) as *mut c_void;
        
        hook.set_cbd(ud);
        
        unsafe {
            hook.set((self.c_hook_server)(self,
                                          cbuf!(name),
                                          pri as i32,
                                          c_callback,
                                          ud));
        }
        hook
    }

    /// Registers a callback to be called when a given print event occurs. This
    /// can be any of the text events listed under Settings > Text Events.
    /// Callback needs to be compatible with this signature:
    /// ```
    /// FnMut(&Hexchat, &[String], &mut UserData) -> Eat
    /// ```
    /// # Arguments
    /// * `name`        - The name of the event to listen for.
    /// * `pri`         - The priority of the callback.
    /// * `callback`    - The callback to invoke when the event occurs.
    /// * `user_data`   - The user data that gets passed back to the callback
    ///                   when it's invoked.
    /// # Returns
    /// * A `Hook` object that can be used to deregister the callback. It
    ///   doesn't need to be retained if not needed.
    ///
    pub fn hook_print<F: 'static>(&self,
                                  event_name  : &str,
                                  pri         : Priority,
                                  callback    : F,
                                  user_data   : UserData
                                 ) -> Hook
    where 
        F: FnMut(&Hexchat, &[String], &mut UserData) -> Eat
    {
        let hook = Hook::new();
        let ud   = Box::new(
                    CallbackData::new_print_data( 
                                      Box::new(callback),
                                      user_data,
                                      hook.clone()
                                  ));
        let ud = Box::into_raw(ud) as *mut c_void;
        
        hook.set_cbd(ud);
        
        unsafe {
            hook.set((self.c_hook_print)(self,
                                         cbuf!(event_name),
                                         pri as i32,
                                         c_print_callback,
                                         ud));
        }
        hook
    }

    /// Registers a callback to be called when a given print event occurs.
    /// The callback will be invoked with an `EventAttrs` object containing
    /// a `time_t` value for the event. The callback needs to be compatible
    /// with this signature:
    /// ```
    /// FnMut(&Hexchat, &[String], &EventAttrs, &mut UserData)
    /// -> Eat
    /// ```
    /// # Arguments
    /// * `name`        - The name of the event to listen for.
    /// * `pri`         - The priority of the callback.
    /// * `callback`    - The callback to invoke when the event occurs.
    /// * `user_data`   - The user data that gets passed back to the callback
    ///                   when it's invoked.
    /// # Returns
    /// * A `Hook` object that can be used to deregister the callback. It
    ///   doesn't need to be retained if not needed.
    ///
    pub fn hook_print_attrs<F: 'static>(&self,
                                        name        : &str,
                                        pri         : Priority,
                                        callback    : F,
                                        user_data   : UserData
                                       ) -> Hook
    where 
        F: FnMut(&Hexchat, &[String], &EventAttrs, &mut UserData)
           -> Eat
    {
        let hook = Hook::new();
        let ud   = Box::new(
                    CallbackData::new_print_attrs_data(
                                      Box::new(callback),
                                      user_data,
                                      hook.clone()
                                  ));
        let ud = Box::into_raw(ud) as *mut c_void;
        
        hook.set_cbd(ud);
        
        unsafe {
            hook.set((self.c_hook_print_attrs)(self,
                                               cbuf!(name),
                                               pri as i32,
                                               c_print_attrs_callback,
                                               ud));
        }
        hook
    }


    /// Sets up a callback to be invoked every `timeout` milliseconds. The
    /// callback needs to be compatible with:
    /// ```
    /// FnMut(&Hexchat, &mut UserData) -> i32
    /// ```
    /// # Arguments
    /// * `timeout`     - The timeout in milliseconds.
    /// * `callback`    - The `FnOnce()` callback.
    /// * `user_data`   - User data included with the callback and passed back
    ///                   to the callback during invocation.
    /// # Returns
    /// * A `Hook` object that is can be used to deregister the callback.
    ///
    pub fn hook_timer<F: 'static>(&self,
                                  timeout   : i64,
                                  callback  : F,
                                  user_data : UserData
                                 ) -> Hook
    where 
        F: FnMut(&Hexchat, &mut UserData) -> i32
    {
        let hook = Hook::new();
        let ud   = Box::new(CallbackData::new_timer_data(
                                            Box::new(callback),
                                            user_data,
                                            hook.clone()
                                        ));
        let ud = Box::into_raw(ud) as *mut c_void;
        
        hook.set_cbd(ud);
        
        unsafe {
            hook.set((self.c_hook_timer)(self,
                                         timeout as c_int,
                                         c_timer_callback,
                                         ud));
        }
        hook
    }

    /// This is a special case feature, used internally to enable other threads
    /// to invoke callbacks on the main thread. This function isn't exported
    /// with the rest of the functions of this class.
    /// # Arguments
    /// * `timeout`     - The timeout in milliseconds.
    /// * `callback`    - The `FnOnce()` callback.
    /// * `user_data`   - User data included with the callback and passed back
    ///                   to the callback during invocation.
    /// # Returns
    /// * A `Hook` object that is used to deregister the callback after it's
    ///   invoked.
    ///
    pub (crate)
    fn hook_timer_once(&self,
                       timeout   : i64,
                       callback  : Box<TimerCallbackOnce>,
                       user_data : UserData
                      ) -> Hook
    {
        // TODO - Put the function signatures somewhere logical (?)

        let hook = Hook::new();
        let ud   = Box::new(CallbackData::new_timer_once_data(
                                            callback,
                                            user_data,
                                            hook.clone()
                                        ));
        let ud = Box::into_raw(ud) as *mut c_void;
        
        hook.set_cbd(ud);
        
        unsafe {
            hook.set((self.c_hook_timer)(self,
                                         timeout as c_int,
                                         c_timer_callback_once,
                                         ud));
        }
        hook
    }                                   

    /// Registers a callback to be called after the given timeout.
    pub fn hook_fd<F: 'static>(&self,
                               fd        : i32,
                               flags     : i32,
                               callback  : F,
                               user_data : UserData
                              ) -> Hook
    where 
        F: FnMut(&Hexchat, i32, i32, &mut UserData) -> Eat
    {
        let hook = Hook::new();
        let ud   = Box::new(CallbackData::new_fd_data(
                                            Box::new(callback),
                                            user_data,
                                            hook.clone()
                                        ));
        let ud = Box::into_raw(ud) as *mut c_void;
        
        hook.set_cbd(ud);
        
        unsafe {
            hook.set((self.c_hook_fd)(self,
                                      fd as c_int,
                                      flags as c_int,
                                      c_fd_callback,
                                      ud));
        }
        hook
    }

    /// Unhooks any Hook that was returned from a callback registration.
    /// Ownership of the user_data is transferred to the caller.
    /// Note: Hexchat unhooks all hooks automatically when a plugin is unloaded,
    /// so the client plugin doesn't have to in that case.
    /// # Arguments
    /// * `hook` - The callback hook to deregister with Hexchat.
    /// # Returns
    /// * The user data that was registered with the callback using one of the
    ///   hook commands. Ownership of this object is transferred to the caller.
    ///
    pub fn unhook(&self, hook: &mut Hook) -> UserData
    {
        hook.unhook()
    }


    /// Issues one of the Hexchat IRC events. The command works for any of the
    /// events listed in Settings > Text Events dialog.
    /// # Arguments
    /// * `event_name`  - The name of the Hexchat text event to send.
    /// * `var_args`    - A slice of `&str`'s containing the event's arguments.
    /// # Returns
    /// * On success, `Ok(())` is returned; otherwise, `Err(<HexchatError>)`.
    ///
    pub fn emit_print(&self, event_name: &str, var_args: &[&str])
        -> Result<(), HexchatError>
    {
        let event_attrs = EventAttrs { server_time_utc: 0 };
        self.emit_print_impl(0, &event_attrs, event_name, var_args)
    }

    
    /// Issues one of the Hexchat IRC events. The command works for any of the
    /// events listed in Settings > Text Events dialog.
    /// # Arguments
    /// * `event_attrs` - A reference to an `EventAttrs` struct.
    /// * `event_name`  - The name of the Hexchat text event to send.
    /// * `var_args`    - A slice of `&str`'s containing the event's arguments.
    /// # Returns
    /// * On success, `Ok(())` is returned; otherwise, `Err(<HexchatError>)`.
    ///
    pub fn emit_print_attrs(&self,
                            event_attrs : &EventAttrs,
                            event_name  : &str,
                            var_args    : &[&str]
                           ) -> Result<(), HexchatError>
    {
        self.emit_print_impl(1, event_attrs, event_name, var_args)
    }

    /// Issues one of the Hexchat IRC events. Called internally by the public
    /// commands, `emit_print()` and `emit_print_attrs()`. The command works
    /// for any of the events listed in Settings > Text Events dialog.
    /// # Arguments
    /// * `ver`         - 0 to invoke `hc.c_emit_print()`, 1 to invoke
    ///                   `hc.c_emit_print_attrs()`.
    /// * `event_attrs` - A reference to an `EventAttrs` struct.
    /// * `event_name`  - The name of the Hexchat text event to send.
    /// * `var_args`    - A slice of `&str`'s containing the event's arguments.
    /// # Returns
    /// * On success, `Ok(())` is returned; otherwise, `Err(<HexchatError>)`.
    ///
    fn emit_print_impl(&self,
                       ver          : i32,
                       event_attrs  : &EventAttrs,
                       event_name   : &str,
                       var_args     : &[&str]
                      ) -> Result<(), HexchatError>
    {
        let mut args   = vec![];
        let     name   = str2cstring(event_name);
        let     va_len = var_args.len();
        
        // We don't know if there are 6 items in var_args - so Clippy's 
        // suggestion would fail. This range loop is fine.
        #[allow(clippy::needless_range_loop)]
        for i in 0..6 {
            args.push(str2cstring(if i < va_len { var_args[i] } else { "" }));
        }
       
        // TODO - If empty strings don't suffice as a nop param, then construct
        //        another vector containing pointers and pad with nulls.
        unsafe {
            use HexchatError::*;
            
            if ver == 0 {
                let result = (self.c_emit_print)(
                                    self,
                                    name.as_ptr(), args[0].as_ptr(),
                                    args[1].as_ptr(), args[2].as_ptr(),
                                    args[3].as_ptr(), args[4].as_ptr(),
                                    args[5].as_ptr(), null::<c_char>());
                if result > 0 {
                    Ok(())
                } else {
                    Err(CommandFailed(format!("`.emit_print(\"{}\", {:?})` \
                                               failed. Check the event name \
                                               and data for errors.",
                                              event_name, var_args)))
                }
            } else {
                let result = (self.c_emit_print_attrs)(
                                    self, event_attrs,
                                    name.as_ptr(), args[0].as_ptr(),
                                    args[1].as_ptr(), args[2].as_ptr(),
                                    args[3].as_ptr(), args[4].as_ptr(),
                                    args[5].as_ptr(), null::<c_char>());
                if result > 0 {
                    Ok(())
                } else {
                    Err(CommandFailed(format!(
                                    "`.emit_print_attrs(\"{}\", {:?})` \
                                     failed. Check the event name \
                                     and data for errors.",
                                    event_name, var_args)))
                }
            }
        }
    }

    /// Compares two nicknames, returning a similar value to `strcmp()`.
    /// If they're equal (0), s1 < s2 (<0 - negative), or s1 > s2 (>0 positive).
    /// # Arguments
    /// * `s1` - The first nickname to compare.
    /// * `s2` - The second.
    /// # Returns
    /// * If the first non-matching character is of lesser value for `s1`, a
    ///   negative value is returned; if `s1`'s char is greater, then a non-0
    ///   postive value is returned. 0 is returned if they match.
    ///
    pub fn nickcmp(&self, s1: &str, s2: &str) -> i32 {
        unsafe {
            (self.c_nickcmp)(self, cbuf!(s1), cbuf!(s2))
        }
    }

    /// Converts a string with text attributes and IRC colors embedded into
    /// a plain text string. Either IRC colors, or text attributes (or both)
    /// can be stripped out of the string.
    /// # Arguments
    /// * `text`    - The string to strip.
    /// * `flags`   - One of the `StripFlags` cases (`StripMIrcColors`,
    ///               `StripTextAttributes`, `StripBoth`).
    /// # Returns
    /// * `Some(<stripped-string>)` or `None` if the operation failed.
    ///
    pub fn strip(&self, text: &str, flags: StripFlags) -> Option<String> {
        let length = text.len() as i32;
        let result = unsafe {
            (self.c_strip)(self, cbuf!(text), length, flags as i32)
        };
        if !result.is_null() {
            let stripped = pchar2string(result);
            unsafe { (self.c_free)(self, result as *const c_void); }
            Some(stripped)
        } else { None }
    }

    /// Sets the currently active context to that bound to the  `Context`
    /// object. The contexts are essentially the channels the user is in
    /// and has open tabs/windows to them. The `Context` object itself has
    /// a `.set()` method that can be invoked directly, which this command
    /// invokes.
    /// # Arguments
    /// * `context` - The `Context` to make the currently active context.
    /// # Returns
    /// * A result (`Result<(), ContextError`) where `Ok(())` indicates
    ///   the context has been switched, and a `ContextError` if it didn't.
    ///
    pub fn set_context(&self, context: &Context) -> Result<(), ContextError> {
        context.set()
    }

    /// Returns a `Context` object bound to the requested server/channel.
    /// The object provides methods like `print()` that will execute the 
    /// Hexchat print command in that tab/window related to the context.
    /// The `Context::find()` can also be invoked to find a context.
    /// # Arguments
    /// * `network`  - The network (e.g. "freenode") of the context.
    /// * `channel`  - The channel name for the context (e.g. "##rust").
    /// # Returns
    /// *  the context was found, i.e. if the user is joined to the channel
    ///    specified currently, a `Some(<Context>)` is returned with the
    ///    context object; `None` otherwise.
    ///
    pub fn find_context(&self, network: &str, channel: &str)
        -> Option<Context>
    {
        Context::find(network, channel)
    }

    /// Returns a `Context` object for the current context (Hexchat tab/window
    /// currently visible in the app). This object can be used to invoke
    /// the Hexchat API within the context the object is bound to. Also,
    /// `Context::get()` will return a context object for the current context.
    /// # Returns
    /// * The `Context` for the currently active context. This usually means
    ///   the channel window the user has visible in the GUI.
    ///
    pub fn get_context(&self) -> Option<Context> {
        Context::get()
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
        let info = unsafe { (self.c_get_info)(self, cbuf!(id)) };
        if !info.is_null() {
            match id {
                "win_ptr"  | "gtkwin_ptr"  => { 
                    Some((info as u64).to_string())
                },
                _ => { 
                    Some(pchar2string(info))
                },
            }
        } else { None }
    }

    /// Returns the requested pref value, or None if it doesn't exist. These
    /// are settings specific to Hexchat itself. It's possible to get the
    /// user's input box text cursor position via this command with
    /// "state_cursor", for instance. Other preferences can be listed with the
    /// `/set` command.
    /// # Arguments
    /// * name - The name of the pref to read.
    /// # Returns
    /// * `Some(PrefValue)` if the pref exists, `None` otherwise.
    ///
    pub fn get_prefs(&self, name: &str) -> Option<PrefValue> {
        unsafe {
            let mut str_ptr: *const c_char = ptr::null();
            let mut int_loc: c_int = 0;
            let result = (self.c_get_prefs)(self,
                                            cbuf!(name),
                                            &mut str_ptr,
                                            &mut int_loc);
            match result {
                1 => { Some(StringVal(pchar2string(str_ptr))) },
                2 => { Some(IntegerVal(int_loc as i32)) },
                3 => { Some(BoolVal( int_loc != 0 )) },
                _ => { None },
            }
        }
    }

    /// Creates an iterator for the requested Hexchat list. This is modeled
    /// after how Hexchat implements the listing feature: rather than load
    /// all the list items up front, an internal list pointer is advanced
    /// to the current item, and the fields of which are accessible through
    /// the iterator's `.get_field()` function. List iterators can also be
    /// created by invoking `ListIterator::new(<list-name>)`. See the Hexchat
    /// Plugin Interface web page for more information on the related lists.
    /// # Arguments
    /// * `name` - The name of the list to iterate over.
    /// # Returns
    /// * If the list exists, `Some(ListIterator)` is returned; `None`
    ///   otherwise.
    ///
    pub fn list_get(&self, name: &str) -> Option<ListIterator> {
        ListIterator::new(name)
    }
    
    /// Writes a variable name and value to a configuration file maintained
    /// by Hexchat for your plugin. These can be accessed later using 
    /// `pluginpref_get()`. *A character representing the type of the pref is
    /// prepended to the value output to the config file. `pluginpref_get()`
    /// uses this when reading back values from the config file to return the
    /// correct variant of `PrefValue`.*
    /// # Arguments
    /// * `name`    - The name of the pref to set.
    /// * `value`   - The value to set - an instance of one of the `PrefValue`
    ///               types (`StringVal, IntVal, or BoolVal`).
    /// # Returns
    /// * `true` if the operation succeeds, `false` otherwise.
    ///
    pub fn pluginpref_set(&self, name: &str, value: &PrefValue) -> bool {
        let sval = value.simple_ser();
        if sval.len() > MAX_PREF_VALUE_SIZE {
            panic!("`hexchat.pluginpref_set({}, <overflow>)`: the value \
                    exceeds the max allowable size of {:?} bytes.",
                   name, MAX_PREF_VALUE_SIZE);
        }
        unsafe {
            (self.c_pluginpref_set_str)(self,
                                        cbuf!(name),
                                        cbuf!(sval.as_str())) > 0
        }
    }

    /// Retrieves, from a config file that Hexchat manages for your plugin,
    /// the value for the named variable that had been previously created using
    /// `pluginpref_set()`.
    /// # Arguments
    /// * `name` - The name of the pref to load.
    /// # Returns
    /// * `Some(<PrefValue>)` holding the value of the requested pref if it
    ///   exists, `None` otherwise.
    ///
    pub fn pluginpref_get(&self, name: &str) -> Option<PrefValue> {
        let mut buf = [0i8; MAX_PREF_VALUE_SIZE];
        if unsafe { (self.c_pluginpref_get_str)(self,
                                                cbuf!(name),
                                                buf.as_mut_ptr()) > 0 }
        {
            let sval = pchar2string(buf.as_ptr());
            Some(PrefValue::simple_deser(&sval))
        } else { None }
    }

    /// Returns a list of all the plugin pref variable names your plugin 
    /// registered using `pluginpref_set()`. `pluginpref_get()` can be invoked
    /// with each item to get their values.
    /// # Returns
    /// * `Some(Vec<String>)` if prefs exist for the plugin, `None` otherwise.
    ///    The vector contains the names of the prefs registered.
    ///
    pub fn pluginpref_list(&self) -> Option<Vec<String>> {
        let mut buf = [0i8; MAX_PREF_LIST_SIZE];
        if unsafe { (self.c_pluginpref_list)(self, buf.as_mut_ptr()) > 0 } {
            let s = pchar2string(buf.as_ptr());
            if !s.is_empty() {
                let mut v = vec![];
                for name in s.split(',') {
                    if !name.is_empty() {
                        v.push(name.to_string());
                    }
                }
                Some(v)
            } else { None }
        } else { None }
    }

    /// Adds a dummy entry in Hexchat's list of plugins. The "plugin" registered
    /// using this command is visible in the "Plugins and Scripts" dialog and
    /// using other slash "/" commands; however, that's all it does. This
    /// command is useful when embedding a script interpreter that loads
    /// scripts as plugin code. Each script thus loaded can be visible to the
    /// user as a plugin. If writing a native plugin, you don't need to be
    /// concerned with this command as your plugin's info is registered during
    /// init from the `PluginInfo` object provided by your `plugin_get_info()`
    /// function.
    /// # Arguments
    /// * `filename`    - This can be the name of a script or binary.
    /// * `name`        - The name of the plugin.
    /// * `desc`        - The description of the plugin.
    /// * `version`     - A version string.
    /// # Returns
    /// * A new `Plugin` object that represents the plugin entry in Hexchat.
    ///   It can be used to deregister the plugin, and it (or a clone of it)
    ///   needs to be retained; otherwise, the plugin entry will be removed
    ///   when the last copy of the `Plugin` object goes out of scope.
    ///
    pub fn plugingui_add(&self,
                         filename : &str,
                         name     : &str,
                         desc     : &str,
                         version  : &str,
                        ) -> Plugin
    {
        Plugin::new(filename, name, desc, version)
    }

    /// Removes the dummy plugin entry from the Hexchat environment. The
    /// dummy plugin would have been registered using `hexchat.plugingui_add()`.
    ///
    pub fn plugingui_remove(&self, plugin: &Plugin) {
        plugin.remove();
    }
}

/// Represents the values that can be accessed using the prefs functions of
/// the `Hexchat` object (`hc.pluginpref_get()`, `hc.pluginpref_get()`, etc.).
/// The enumeration enables the typing of the values stored and retrieved.
///
#[derive(Debug)]
pub enum PrefValue {
    StringVal(String),
    IntegerVal(i32),
    BoolVal(bool),
}
use PrefValue::*;

impl PrefValue {
    /// Simple config file value serialization into string.
    /// The string produced can be written to the config file Hexchat maintains.
    /// A type character is prepended ('s', 'i', or 'b').
    ///
    fn simple_ser(&self) -> String {
        match self {
            StringVal(s) => {
                let mut sstr = s.clone();
                sstr.insert(0, 's');
                sstr
            },
            IntegerVal(i) => {
                let mut istr = i.to_string();
                istr.insert(0, 'i');
                istr
            },
            BoolVal(b) => {
                let mut bstr = b.to_string();
                bstr.insert(0, 'b');
                bstr
            },
        }
    }
    /// Simple config file value deserialization from a string to a `PrefValue`.
    /// Treats the first character of the string read in from the config file
    /// as the type, which it then discards and parses the rest of the string
    /// to return the correct variant of `PrefValue`.
    ///
    fn simple_deser(s: &str) -> PrefValue {
        if s.len() > 1 {
            match &s[0..1] {
                "s" => {
                    StringVal(s.to_string())
                },
                "i" => {
                    if let Ok(v) = s[1..].parse::<i32>() {
                        IntegerVal(v)
                    } else { StringVal(s.to_string()) }
                },
                "b" => {
                    if let Ok(v) = s[1..].parse::<bool>() {
                        BoolVal(v)
                    } else { StringVal(s.to_string()) }
                },
                _ => { StringVal(s.to_string()) },
            }
        } else {
            StringVal(s.to_string())
        }
    }
}

/// Some types used by the C struct below.
#[allow(non_camel_case_types)]
pub (crate) type hexchat_hook        = c_void;
#[allow(non_camel_case_types)]
pub (crate) type hexchat_list        = c_void;
#[allow(non_camel_case_types)]
pub (crate) type hexchat_context     = c_void;
#[allow(dead_code, non_camel_case_types)]
pub (crate) type hexchat_event_attrs = c_void;

/// Mirrors the callback function pointer of Hexchat.
#[allow(non_camel_case_types)]
type C_Callback      = extern "C"
                       fn(word       : *const *const c_char,
                          word_eol   : *const *const c_char,
                          user_data  : *mut c_void
                         ) -> c_int;

/// Mirrors the print callback function pointer of Hexchat.
#[allow(non_camel_case_types)]
type C_PrintCallback = extern "C"
                       fn(word       : *const *const c_char,
                          user_data  : *mut c_void
                         ) -> c_int;

/// Mirrors the timer callback function pointer of Hexchat.
#[allow(non_camel_case_types)]
type C_TimerCallback = extern "C"
                       fn(user_data  : *mut c_void
                         ) -> c_int;

/// Mirrors the print attr callback function pointer of Hexchat.
#[allow(non_camel_case_types)]
type C_AttrCallback  = extern "C"
                       fn(word       : *const *const c_char,
                          attrs      : *const EventAttrs,
                          user_data  : *mut c_void
                         ) -> c_int;

/// Mirrors the FD related callback function pointer of Hexchat.
#[allow(non_camel_case_types)]
type C_FDCallback    = extern "C"
                       fn(fd         : c_int,
                          flags      : c_int,
                          udata      : *mut c_void
                         ) -> c_int;

/// Mirrors the C struct for `hexchat_event_attrs`. It holds the timestamps
/// for the callback invocations for callbacks registered using 
/// `hexchat_print_attrs()`, and similar commands.
#[repr(C)]
pub struct EventAttrs {
    pub server_time_utc : time_t
}

impl fmt::Debug for Hexchat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hexchat")
            .field("raw_function_pointers", &"...")
            .finish()
    }
}

/// Errors generated directly from the main Object, `Hexchat`.
#[derive(Debug)]
pub enum HexchatError {
    CommandFailed(String),
}

impl error::Error for HexchatError {}

impl fmt::Display for HexchatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use HexchatError::*;
        match self {
            CommandFailed(message) => {
                write!(f, "CommandFailed(\"{}\")", message)
            },
        }
    }
}
/*
impl Error for HexchatError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.side)
    }
}
*/


/// This struct mirrors the C Hexchat struct passed to the plugin from
/// Hexchat when the plugin is loaded. Hexchat's API is implemented as a struct
/// holding callbacks to its native functions. *Don't modify* this struct,
/// unless there has been a change to the layout of it in the Hexchat C code
/// base.
#[repr(C)]
pub struct Hexchat {
    pub (crate)
    c_hook_command       : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              name   : *const c_char,
                              pri    : c_int,
                              cb     : C_Callback,
                              help   : *const c_char,
                              udata  : *mut c_void
                             ) -> *const hexchat_hook,
    pub (crate)
    c_hook_server        : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              name   : *const c_char,
                              pri    : c_int,
                              cb     : C_Callback,
                              udata  : *mut c_void
                             ) -> *const hexchat_hook,
    pub (crate)
    c_hook_print         : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              name   : *const c_char,
                              pri    : c_int,
                              cb     : C_PrintCallback,
                              udata  : *mut c_void
                             ) -> *const hexchat_hook,
    pub (crate)
    c_hook_timer         : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              timeout: c_int,
                              cb     : C_TimerCallback,
                              udata  : *mut c_void
                             ) -> *const hexchat_hook,
    pub (crate)
    c_hook_fd            : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              fd     : c_int,
                              flags  : c_int,
                              cb     : C_FDCallback,
                              udata  : *mut c_void
                             ) -> *const hexchat_hook,
    pub (crate)
    c_unhook             : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              hook   : *const hexchat_hook,
                             ) -> *const c_void,
    pub (crate)
    c_print              : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              text   : *const c_char),
    pub (crate)
    c_printf             : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              text   : *const c_char,
                              ...),
    pub (crate)
    c_command            : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              command: *const c_char),
    pub (crate)
    c_commandf           : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              command: *const c_char,
                              ...),
    pub (crate)
    c_nickcmp            : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              s1     : *const c_char,
                              s2     : *const c_char
                             ) -> c_int,
    pub (crate)
    c_set_context        : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              ctx    : *const hexchat_context
                             ) -> c_int,
    pub (crate)
    c_find_context       : unsafe extern "C"
                           fn(hp         : *const Hexchat,
                              srv_name   : *const c_char,
                              channel    : *const c_char,
                             ) -> *const hexchat_context,
    pub (crate)
    c_get_context        : unsafe extern "C"
                           fn(hp: *const Hexchat
                             ) -> *const hexchat_context,
    pub (crate)
    c_get_info           : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              id     : *const c_char,
                             ) -> *const c_char,
    pub (crate)
    c_get_prefs          : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              name   : *const c_char,
                              string : *mut *const c_char,
                              integer: *mut c_int
                             ) -> c_int,
    pub (crate)
    c_list_get           : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              name   : *const c_char
                             ) -> *const hexchat_list,
    pub (crate)
    c_list_free          : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              hclist : *const hexchat_list
                             ),
    pub (crate)
    c_list_fields        : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              name   : *const c_char
                             ) -> *const *const c_char,
    pub (crate)
    c_list_next          : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              hclist : *const hexchat_list
                             ) -> c_int,
    pub (crate)
    c_list_str           : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              hclist : *const hexchat_list,
                              field  : *const c_char
                             ) -> *const c_char,
    pub (crate)
    c_list_int           : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              hclist : *const hexchat_list,
                              field  : *const c_char
                             ) -> c_int,
    pub (crate)
    c_plugingui_add      : unsafe extern "C"
                           fn(hp         : *const Hexchat,
                              filename   : *const c_char,
                              name       : *const c_char,
                              desc       : *const c_char,
                              version    : *const c_char,
                              reserved   : *const c_char
                             ) -> *const c_void,
    pub (crate)
    c_plugingui_remove   : unsafe extern "C"
                           fn(hp: *const Hexchat, handle: *const c_void),
    pub (crate)
    c_emit_print         : unsafe extern "C"
                           fn(hp         : *const Hexchat,
                              event_name : *const c_char,
                              ...
                             ) -> c_int,
    pub (crate)
    c_read_fd            : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              src    : *const c_void,
                              buf    : *mut c_char,
                              len    : *mut c_int
                              ) -> c_int,
    pub (crate)
    c_list_time          : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              hclist : *const hexchat_list,
                              name   : *const c_char
                             ) -> time_t,
    pub (crate)
    c_gettext            : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              msgid  : *const c_char
                             ) -> *const c_char,
    pub (crate)
    c_send_modes         : unsafe extern "C"
                           fn(hp             : *const Hexchat,
                              targets        : *const *const c_char,
                              n_targets      : c_int,
                              modes_per_line : c_int,
                              sign           : c_char,
                              mode           : c_char
                             ),
    pub (crate)
    c_strip              : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              string : *const c_char,
                              len    : c_int,
                              flags  : c_int
                             ) -> *const c_char,
    pub (crate)
    c_free               : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              ptr    : *const c_void,
                             ),
    pub (crate)
    c_pluginpref_set_str : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              var    : *const c_char,
                              value  : *const c_char
                             ) -> c_int,
    pub (crate)
    c_pluginpref_get_str : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              var    : *const c_char,
                              dest   : *mut c_char
                             ) -> c_int,
    pub (crate)
    c_pluginpref_set_int : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              var    : *const c_char,
                              value  : c_int
                             ) -> c_int,
    pub (crate)
    c_pluginpref_get_int : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              var    : *const c_char
                             ) -> c_int,
    pub (crate)
    c_pluginpref_delete  : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              var    : *const c_char
                             ) -> c_int,
    pub (crate)
    c_pluginpref_list    : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              dest   : *mut c_char
                             ) -> c_int,
    pub (crate)
    c_hook_server_attrs  : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              name   : *const c_char,
                              pri    : c_int,
                              cb     : C_AttrCallback,
                              udata  : *const c_void
                             ) -> *const hexchat_hook,
    pub (crate)
    c_hook_print_attrs   : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              name   : *const c_char,
                              pri    : c_int,
                              cb     : C_AttrCallback,
                              udata  : *const c_void
                             ) -> *const hexchat_hook,
    pub (crate)
    c_emit_print_attrs   : unsafe extern "C"
                           fn(hp         : *const Hexchat,
                              attrs      : *const EventAttrs,
                              event_name : *const c_char,
                              ...
                             ) -> c_int,
    pub (crate)
    c_event_attrs_create : unsafe extern "C"
                           fn(hp: *const Hexchat) -> *mut EventAttrs,
    pub (crate)
    c_event_attrs_free   : unsafe extern "C"
                           fn(hp     : *const Hexchat,
                              attrs  : *mut EventAttrs
                             ),
}
