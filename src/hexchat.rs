#![allow(non_camel_case_types, dead_code, unused_macros)]

//! This file holds the Hexchat struct used to interface with Hexchat. Its
//! fields are the actual callbacks provided by Hexchat. When Hexchat
//! loads this library, the Hexchat pointer is stored and used by casting it
//! to the struct contained in this file. These native function pointers are
//! private to this file, and a more Rust-friendly API is provided through
//! this Hexchat interface. 

use libc::{c_int, c_char, c_void, time_t};
use std::any::Any;
use std::convert::From;
use std::{error, ptr};
use std::ffi::{CString, CStr};
use std::fmt;
use std::ops::BitOr;
use std::ops::FnMut;
use std::ptr::null;
use std::str;

use crate::callback_data::{CallbackData, TimerCallbackOnce};
use crate::context::Context;
use crate::context::ContextError;
use crate::hexchat_callbacks::*;
use crate::hook::Hook;
use crate::list_iterator::ListIterator;
use crate::plugin::Plugin;
use crate::utils::*;

use crate::HexchatError::*;

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

pub enum StripFlags {
    StripMIrcColors     = 1,
    StripTextAttributes = 2,
    StripBoth           = 3,
}

/// This is the rust-facing Hexchat API. Each method has a corresponding
/// C function pointer which they wrap and marshal data/from.
/// 
impl Hexchat {

    /// Prints the string passed to it to the active Hexchat window.
    ///
    /// # Arguments
    /// * `text` - The text to print.
    ///
    pub fn print(&self, text: &str) {
        unsafe { (self.c_print)(self, cbuf!(text)); }
    }

    /// Invokes the Hexchat command specified by `command`.
    ///
    /// # Arguments
    /// * `command` - The Hexchat command to invoke.
    ///
    pub fn command(&self, command: &str) {
        unsafe { (self.c_command)(self, cbuf!(command)); }
    }

    /// Registeres a command callback with Hexchat.
    ///
    /// The callback can be a static function, or a closure, that has the form:
    /// 
    /// ```
    ///     FnMut(&Hexchat, &[String], &[String], &mut Option<Box<dyn Any>>) 
    ///     -> Eat
    /// ```
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
                                    user_data   : Option<Box<dyn Any>>
                                   ) -> Hook
    where 
        F: FnMut(&Hexchat, &[String], &[String], &mut Option<Box<dyn Any>>) 
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
        unsafe {
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
    pub fn hook_server<F: 'static>(&self,
                                   name        : &str,
                                   pri         : Priority,
                                   callback    : F,
                                   user_data   : Option<Box<dyn Any>>
                                  ) -> Hook
    where 
        F: FnMut(&Hexchat, &[String], &[String], &mut Option<Box<dyn Any>>) 
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
        unsafe {
            hook.set((self.c_hook_server)(self,
                                          cbuf!(name),
                                          pri as i32,
                                          c_callback,
                                          ud));
        }
        hook
    }

    /// Registers a callback to be called when a given print event occurs.
    pub fn hook_print<F: 'static>(&self,
                                  event_name  : &str,
                                  pri         : Priority,
                                  callback    : F,
                                  user_data   : Option<Box<dyn Any>>
                                 ) -> Hook
    where 
        F: FnMut(&Hexchat, &[String], &mut Option<Box<dyn Any>>) -> Eat
    {
        let hook = Hook::new();
        let ud   = Box::new(
                    CallbackData::new_print_data( 
                                      Box::new(callback),
                                      user_data,
                                      hook.clone()
                                  ));
        let ud = Box::into_raw(ud) as *mut c_void;
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
    pub fn hook_print_attrs<F: 'static>(&self,
                                        name        : &str,
                                        pri         : Priority,
                                        callback    : F,
                                        user_data   : Option<Box<dyn Any>>
                                       ) -> Hook
    where 
        F: FnMut(&Hexchat, &[String], &EventAttrs, &mut Option<Box<dyn Any>>) 
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
        unsafe {
            hook.set((self.c_hook_print_attrs)(self,
                                               cbuf!(name),
                                               pri as i32,
                                               c_print_attrs_callback,
                                               ud));
        }
        hook
    }


    /// Registers a callback to be called after the given timeout.
    pub fn hook_timer<F: 'static>(&self,
                                  timeout   : i64,
                                  callback  : F,
                                  user_data : Option<Box<dyn Any>>
                                 ) -> Hook
    where 
        F: FnMut(&Hexchat, &mut Option<Box<dyn Any>>) -> i32
    {
        let hook = Hook::new();
        let ud   = Box::new(CallbackData::new_timer_data(
                                            Box::new(callback),
                                            user_data,
                                            hook.clone()
                                        ));
        let ud = Box::into_raw(ud) as *mut c_void;
        unsafe {
            hook.set((self.c_hook_timer)(self,
                                         timeout as c_int,
                                         c_timer_callback,
                                         ud));
        }
        hook
    }

    /// This is a special case feature, used internally to enable other threads
    /// to invoke callbacks on the main thread.
    pub (crate)
    fn hook_timer_once(&self,
                       timeout   : i64,
                       callback  : Box<TimerCallbackOnce>,
                       user_data : Option<Box<dyn Any>>
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
                               user_data : Option<Box<dyn Any>>
                              ) -> Hook
    where 
        F: FnMut(&Hexchat, i32, i32, &mut Option<Box<dyn Any>>) -> Eat
    {
        let hook = Hook::new();
        let ud   = Box::new(CallbackData::new_fd_data(
                                            Box::new(callback),
                                            user_data,
                                            hook.clone()
                                        ));
        let ud = Box::into_raw(ud) as *mut c_void;
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
    pub fn unhook(&self, hook: &mut Hook) -> Option<Box<dyn Any>> 
    {
        hook.unhook()
    }


    /// Issues one of the Hexchat IRC events. The command works for any of the
    /// events listed in Settings > Text Events dialog.
    pub fn emit_print(&self, event_name: &str, var_args: &[&str])
        -> Result<(), HexchatError>
    {
        let event_attrs = EventAttrs { server_time_utc: 0 };
        self.emit_print_impl(0, &event_attrs, event_name, var_args)
    }

    
    /// Issues one of the Hexchat IRC events. The command works for any of the
    /// events listed in Settings > Text Events dialog.
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
    fn emit_print_impl(&self,
                       ver          : i32,
                       event_attrs  : &EventAttrs,
                       event_name   : &str,
                       var_args     : &[&str]
                      ) -> Result<(), HexchatError>
    {
        let emsg  = "Hexchat.emit_print() string conversion failed.";
        let empty = str2cstring("");

        let mut args = vec![];
        let     name = str2cstring(event_name);

        for i in 0..6 {
            let c_arg;
            if i < var_args.len() {
                c_arg = str2cstring(var_args[i]);
            } else {
                c_arg = empty.clone();
            }
            args.push(c_arg);
        }
        // TODO - If empty strings don't suffice as a nop param, then construct
        //        another vector containing pointers and pad with nulls.
        unsafe {
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

    pub fn nickcmp(&self, s1: &str, s2: &str) -> i32 {
        unsafe {
            (self.c_nickcmp)(self, cbuf!(s1), cbuf!(s2))
        }
    }

    /// Converts a string with text attributes and IRC colors embedded into
    /// a plain text string. Either IRC colors, or text attributes (or both)
    /// can be stripped out of the string.
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
    /// and has open tabs/windows to them.
    pub fn set_context(&self, context: &Context) -> Result<(), ContextError> {
        context.set()
    }

    /// Returns a `Context` object bound to the requested server/channel.
    /// The object provides methods like `print()` that will execute the 
    /// Hexchat print command in that tab/window related to the context.
    pub fn find_context(&self, server: &str, channel: &str)
        -> Option<Context>
    {
        Context::find(server, channel)
    }

    /// Returns a `Context` object for the current context (Hexchat tab/window
    /// currently visible in the app). This object can be used to invoke
    /// the Hexchat API within the context the object is bound to.
    pub fn get_context(&self) -> Option<Context> {
        Context::get()
    }

    // TODO - Combine PrefValue and FieldValue into one enum.
    // TODO - Consider making the common string type CString for compatibility
    //        reasons. No.. Seems that most other crates made for Rust want to
    //        use Rust's native String/str types.

    /// Retrieves the info data with the given `id`. It returns None on failure
    /// and Some(String) on success. All information is returned as String
    /// data - even the "win_ptr"/"gtkwin_ptr" values - these can be easily
    /// converted to a pointer using an approprate parsing function.
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

    /// Returns the requested pref value, or None if it doesn't exist.
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

    pub fn list_get(&self, name: &str) -> Option<ListIterator> {
        ListIterator::new(name)
    }
    
    /// Writes a variable name and value to a configuration file maintained
    /// by Hexchat for your plugin. These can be accessed later using 
    /// `pluginpref_get()`.
    ///
    pub fn pluginpref_set(&self, name: &str, value: &PrefValue) -> bool {
        if let Ok(ser_val) = serde_json::to_string(value) {
            if ser_val.len() > MAX_PREF_VALUE_SIZE {
                panic!("`hexchat.pluginpref_set()` value is larger than the \
                        current buffer can hold when read back. Please \
                        consider splitting the data into parts, or some \
                        other approach to reduce the value size of the \
                        data. The max size is {:?} bytes including \
                        serialization data.", MAX_PREF_VALUE_SIZE);
            }
            unsafe {
                (self.c_pluginpref_set_str)(self,
                                            cbuf!(name),
                                            cbuf!(ser_val)) > 0
            }
        } else {
            false
        }
    }

    /// Retrieves, from a config file that Hexchat manages for your plugin,
    /// the value for the named variable that had been previously created using
    /// `pluginpref_set()`.
    ///
    pub fn pluginpref_get(&self, name: &str) -> Option<PrefValue> {
        let mut buf = [0i8; MAX_PREF_VALUE_SIZE];
        if unsafe { (self.c_pluginpref_get_str)(self,
                                                cbuf!(name),
                                                buf.as_mut_ptr()) > 0 }
        {
            let ser_val = pchar2string(buf.as_ptr());
            if let Ok(val) = serde_json::from_str(&ser_val) {
                Some(val)
            } else {
                None
            }
        } else { None }
    }

    /// Returns a list of all the plugin pref variable names your plugin 
    /// registered using `pluginpref_set()`. `pluginpref_get()` can be invoked
    /// on each item to get their values.
    ///
    pub fn pluginpref_list(&self) -> Option<Vec<String>> {
        let mut buf = [0i8; MAX_PREF_LIST_SIZE];
        if unsafe { (self.c_pluginpref_list)(self, buf.as_mut_ptr()) > 0 } {
            let s = pchar2string(buf.as_ptr());
            if s.len() > 0 {
                let mut v = vec![];
                for name in s.split(",") {
                    if !name.is_empty() {
                        v.push(name.to_string());
                    }
                }
                Some(v)
            } else {
                None
            }
        } else {
            None
        }
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

#[derive(Debug, Deserialize, Serialize)]
pub enum PrefValue {
    StringVal(String),
    IntegerVal(i32),
    BoolVal(bool),
}
use PrefValue::*;

pub (crate) type hexchat_hook        = c_void;
pub (crate) type hexchat_list        = c_void;
pub (crate) type hexchat_context     = c_void;
pub (crate) type hexchat_event_attrs = c_void;

/// Mirrors the callback function pointer of Hexchat.
type C_Callback      = extern "C"
                       fn(word       : *const *const c_char,
                          word_eol   : *const *const c_char,
                          user_data  : *mut c_void
                         ) -> c_int;

/// Mirrors the print callback function pointer of Hexchat.
type C_PrintCallback = extern "C"
                       fn(word       : *const *const c_char,
                          user_data  : *mut c_void
                         ) -> c_int;

/// Mirrors the timer callback function pointer of Hexchat.
type C_TimerCallback = extern "C"
                       fn(user_data  : *mut c_void
                         ) -> c_int;

/// Mirrors the print attr callback function pointer of Hexchat.
type C_AttrCallback  = extern "C"
                       fn(word       : *const *const c_char,
                          attrs      : *const EventAttrs,
                          user_data  : *mut c_void
                         ) -> c_int;

/// Mirrors the FD related callback function pointer of Hexchat.
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
use HexchatError::*;
use std::fmt::Debug;
use std::collections::HashMap;

impl error::Error for HexchatError {}

impl fmt::Display for HexchatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
/// holding callbacks to its native functions.
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
