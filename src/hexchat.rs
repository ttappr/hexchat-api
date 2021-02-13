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
use std::ffi::{CString, CStr};
use std::fmt;
use std::ops::BitOr;
use std::ops::FnMut;
use std::ptr::null;
use std::str;

use crate::callback_data::{CallbackData, TimerCallbackOnce};
use crate::context::Context;
use crate::context::ContextError;
use crate::errors::*;
use crate::hexchat_callbacks::*;
use crate::hook::Hook;
use crate::list_iterator::ListIterator;
use crate::plugin::Plugin;
use crate::utils::*;

use crate::HexchatError::*;

// Note the non-intuitive way macros from other files have to be accessed!
use crate::cbuf;

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
    StripMIrcColors = 1,
    StripTextAttributes = 2,
    StripBoth = 3,
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
                       timeout  : i64,
                       callback : Box<TimerCallbackOnce>,
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


    pub fn emit_print(&self, event_name: &str, var_args: &[&str])
        -> Result<(), HexchatError>
    {
        let event_attrs = EventAttrs { server_time_utc: 0 };
        self.emit_print_impl(0, &event_attrs, event_name, var_args)
    }

    pub fn emit_print_attrs(&self,
                            event_attrs: &EventAttrs,
                            event_name: &str,
                            var_args: &[&str]
                           ) -> Result<(), HexchatError>
    {
        self.emit_print_impl(1, event_attrs, event_name, var_args)
    }

    fn emit_print_impl(&self,
                       ver: i32,
                       event_attrs: &EventAttrs,
                       event_name: &str,
                       var_args: &[&str]
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
                    Err(CommandFailed(format!("`.emit_print(\"{}\", {:?})` \
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

    pub fn set_context(&self, context: &Context) -> Result<(), ContextError> {
        context.set()
    }

    pub fn find_context(&self, server: &str, channel: &str)
        -> Option<Context>
    {
        Context::find(server, channel)
    }

    pub fn get_context(&self) -> Option<Context> {
        Context::get()
    }

    pub fn get_info(&self, id: &str) -> Option<String> {
        let info = unsafe { (self.c_get_info)(self, cbuf!(id)) };
        if !info.is_null() {
            Some(pchar2string(info))
        } else { None }
    }

    pub fn get_prefs(&self, name: &str) -> Result<(), ()> {
        // TODO - Need to implement an error type for this. Or consider Option.
        //        Maybe for strings supplied by the user, it makes more sense
        //        to return Option's because it's expected the user makes errs.
        unimplemented!()
    }

    pub fn list_get(&self, name: &str) -> Option<ListIterator> {
        ListIterator::new(name)
    }

    pub fn plugingui_add(&self,
                         filename : &str,
                         name     : &str,
                         desc     : &str,
                         version  : &str,
                        ) -> Plugin
    {
        Plugin {}
    }
}

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
