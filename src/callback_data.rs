
//#![feature(unsized_locals)] ????

//! The `CallbackData` object holds all the information about a callback 
//! needed to manage the `user_data` and  invoke it safely and correctly.
//! The objects of this module are used internally. This file also contains
//! type declarations for the Rust-facing callback signatures.

use std::any::Any;
use std::mem::ManuallyDrop;

use crate::hook::Hook;
use crate::hexchat::{ Hexchat, Eat, EventAttrs };
use crate::user_data::*;
use crate::utils::*;

use UserData::*;
use core::mem;

use UCallback::*;

/// Holds the Rust-implemented function, or closure, of a registered Hexchat 
/// callback.
///
enum UCallback {
    Command     (Box< Callback >           ),
    Print       (Box< PrintCallback >      ),
    PrintAttrs  (Box< PrintAttrsCallback > ),
    Timer       (Box< TimerCallback >      ),
    TimerOnce   (Box< TimerCallbackOnce >  ),
    FD          (Box< FdCallback >         ),
    OnceDone,
}
impl Default for UCallback {
    /// Supports `mem::take()` for the `TimerOnce` callback invocation.
    /// The value of that callback is replaced with `OnceDone` when 
    /// `mem::take()` is performed on it in `timer_once_cb()`.
    fn default() -> Self { OnceDone }
}


/// Pointers to instances of this struct are registered with the Hexchat
/// callbacks. On the C-facing side, this is the `user_data` passed to the
/// native callback wrappers (like `c_print_callback()`). When invoked by
/// Hexchat, the native callbacks receive a pointer to a `user_data` 
/// (`CallbackData`) object, which the wrapper then uses to invoke the 
/// Rust-implemented callback held in the `UCallback` field below. The `data` 
/// field holds the user data registered for the Rust-facing callback, and is
/// passed to it when invoked.
pub (crate) 
struct CallbackData {
    callback    : UCallback,
    data        : UserData,
    hook        : Hook,
}

impl CallbackData {
    /// Creates callback data for a regular command or server command.
    pub (crate)
    fn new_command_data(callback : Box<Callback>, 
                        data     : UserData,
                        hook     : Hook
                       ) -> Self 
    {
        let callback = Command(callback);
        CallbackData { callback, data, hook  }
    }

    /// Creates callback data for a print callback.
    pub (crate)
    fn new_print_data(callback  : Box<PrintCallback>, 
                      data      : UserData,
                      hook      : Hook
                     ) -> Self
    {
        let callback = Print(callback);
        CallbackData { callback, data, hook }
    }

    /// Creates callback data for a print attrs callback.
    pub (crate)
    fn new_print_attrs_data(callback : Box<PrintAttrsCallback>, 
                            data     : UserData,
                            hook     : Hook
                           ) -> Self
    {
        let callback = PrintAttrs(callback);
        CallbackData { callback, data, hook }
    }

    /// Creates callback data for a timer callback.
    pub (crate)
    fn new_timer_data(callback : Box<TimerCallback>, 
                      data     : UserData,
                      hook     : Hook
                     ) -> Self
    {
        let callback = Timer(callback);
        CallbackData { callback, data, hook }
    }

    pub (crate)
    fn new_timer_once_data(callback : Box<TimerCallbackOnce>,
                           data     : UserData,
                           hook     : Hook
                          ) -> Self
    {
        let callback = TimerOnce(callback);
        CallbackData { callback, data, hook }
    }

    
    /// Creates callback data for a fd callback.
    pub (crate)
    fn new_fd_data(callback : Box<FdCallback>, 
                   data     : UserData,
                   hook     : Hook
                  ) -> Self
    {
        let callback = FD(callback);
        CallbackData { callback, data, hook }
    }

    /// Returns a mutable reference to the Rust-facing `user_data` that was
    /// registered with the callback.    
    #[inline]
    pub (crate)
    fn get_data_mut(&mut self) -> &mut UserData {
        &mut self.data
    }
    
    /// Returns the `data` (Rust facing user_data) field of the object. 
    /// # Returns
    /// * If the user data type was one of the shared types (`SharedData` or
    ///   `SyncData`) a clone will be returned. `NoData` is returned otherwise.
    ///
    pub (crate)
    fn get_data(&self) -> UserData {
        // mem::take(&mut self.data) <- This causes crashes when unloading
        // the plugin. Box's can't clone or copy, so can't return them from 
        // here - A NoData will be returned instead. The other variants will
        // return a clone.
        match self.data {
            BoxedData(_) => { NoData },
            _            => { self.data.clone() },
        }
    }

    /// Invokes the callback held in the `callback` field.
    #[inline]
    pub (crate)
    unsafe fn command_cb(&mut self,
                         hc       : &Hexchat, 
                         word     : &[String], 
                         word_eol : &[String], 
                         ud       : &mut UserData
                        ) -> Eat
    {
        if let Command(callback) = &mut self.callback {
            (*callback)(hc, word, word_eol, ud)
        } else {
            panic!("Invoked wrong type in CallbackData.");
        }
    }
    
    /// Invokes the callback held in the `callback` field. This is invoked by
    /// `c_print_callback()` which is the C-side registered callback for each
    /// print callback.
    #[inline]
    pub (crate)
    unsafe fn print_cb(&mut self,
                       hc       : &Hexchat, 
                       word     : &[String], 
                       ud       : &mut UserData
                      ) -> Eat 
    {
        if let Print(callback) = &mut self.callback {
            (*callback)(hc, word, ud)
        } else {
            panic!("Invoked wrong type in CallbackData.");
        }
    }
    
    /// Invokes the callback held in the `callback` field. This is invoked by
    /// `c_print_attrs_callback()`.
    #[inline]
    pub (crate)
    unsafe fn print_attrs_cb(&mut self,
                             hc    : &Hexchat,
                             word  : &[String],
                             attrs : &EventAttrs,
                             ud    : &mut UserData
                            ) -> Eat
    {
        if let PrintAttrs(callback) = &mut self.callback {
            (*callback)(hc, word, attrs, ud)
        } else {
            panic!("Invoked wrong type in CallbackData.");
        }
    }
    
    /// Invokes the callback held in the `callback` field. This is invoked by
    /// c_timer_callback()`.
    #[inline]
    pub (crate)
    unsafe fn timer_cb(&mut self, hc: &Hexchat, ud: &mut UserData) -> i32
    {
        if let Timer(callback) = &mut self.callback {
            let keep_going = (*callback)(hc, ud);
            if keep_going == 0 {
                self.hook.unhook();
                0
            } else {
                1
            }
        } else {
            panic!("Invoked wrong type in CallbackData.");
        }
    }

    /// One time use timer callback. This is a special case; it's used for
    /// invoking callbacks on the main thread from other threads. It will
    /// unhook itself after one use.
    #[inline]
    pub (crate)
    unsafe fn timer_once_cb(&mut self, hc: &Hexchat, ud: &mut UserData) -> i32
    {
        let variant = mem::take(&mut self.callback);
        match variant {
            TimerOnce(callback) => {
                (callback)(hc, ud);
                self.hook.unhook();
                0
            },
            OnceDone => {
                panic!("Invoked a one-time callback more than once.");
            },
            _ => {
                panic!("Invoked wrong type in CallbackData.");
            },
        }
    }
    

    /// Invokes the callback held in the `callback` field. This is invoked by
    /// c_fd_callback()`.
    #[inline]
    pub (crate)
    unsafe fn fd_cb(&mut self, 
                    hc    : &Hexchat, 
                    fd    : i32, 
                    flags : i32, 
                    ud    : &mut UserData) -> Eat
    {
        if let FD(callback) = &mut self.callback {
            (*callback)(hc, fd, flags, ud)
        } else {
            panic!("Invoked wrong type in CallbackData.");
        }
    }
}

/// The Rust-facing function signature corresponding to the C-facing  
/// `C_Callback`. Note that, unlike the C API, the Rust-facing callback
/// signatures include a reference to the Hexchat pointer for 
/// convenience.
pub (crate)
type Callback = dyn FnMut(&Hexchat,
                          &[String],
                          &[String],
                          &mut UserData
                         ) -> Eat;

/// The Rust-facing function signature corresponding to the C-facing  
/// `C_PrintCallback`. Note that, unlike the C API, the Rust-facing callback
/// signatures include a reference to the Hexchat pointer for 
/// convenience.
pub (crate)
type PrintCallback 
              = dyn FnMut(&Hexchat,
                          &[String],
                          &mut UserData
                         ) -> Eat;

/// The Rust-facing function signature corresponding to the C-facing  
/// `C_PrintAttrsCallback`. Note that, unlike the C API, the Rust-facing callback
/// signatures include a reference to the Hexchat pointer for 
/// convenience.
pub (crate)
type PrintAttrsCallback 
              = dyn FnMut(&Hexchat,
                          &[String],
                          &EventAttrs,
                          &mut UserData
                         ) -> Eat;

/// The Rust-facing function signature corresponding to the C-facing  
/// `C_TimerCallback`. Note that, unlike the C API, the Rust-facing callback
/// signatures include a reference to the Hexchat pointer for 
/// convenience.
pub (crate)
type TimerCallback 
              = dyn FnMut(&Hexchat, &mut UserData) -> i32;
              
pub (crate)
type TimerCallbackOnce 
              = dyn FnOnce(&Hexchat, &mut UserData) -> i32;


/// The Rust-facing function signature corresponding to the C-facing  
/// `C_FdCallback`. Note that, unlike the C API, the Rust-facing callback
/// signatures include a reference to the Hexchat pointer for 
/// convenience.
pub (crate)
type FdCallback 
              = dyn FnMut(&Hexchat, i32, i32, &mut UserData) -> Eat;
              
