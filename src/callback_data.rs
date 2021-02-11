
//! The `CallbackData` object holds all the information about a callback 
//! needed to manage the `user_data` and  invoke it safely and correctly.
//! The objects of this module are used internally. This file also contains
//! type declarations for the Rust-facing callback signatures.

use std::any::Any;
use std::mem::ManuallyDrop;

use crate::hook::Hook;
use crate::hexchat::{ Hexchat, Eat, EventAttrs };
use crate::utils::*;

/// An enumeration of the different types of callback.
#[derive(PartialEq)]
enum CBType { Command, Print, PrintAttrs, Timer, FD }

/// The Rust-facing `user_data` type.
type UserData = Option<Box<dyn Any>>;

/// Holds the Rust-implemented function, or closure, of a registered Hexchat 
/// callback. `ManuallyDrop` had to be applied to the union's fields to get
/// it to compile - it's a 0-cost abstraction, so no big deal.
type MD<T> = ManuallyDrop<Box<T>>;
union UCallback { 
    command        : MD<Callback>,
    print          : MD<PrintCallback>,
    print_attrs    : MD<PrintAttrsCallback>,
    timer          : MD<TimerCallback>,
    fd             : MD<FdCallback>,
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
    cbtype      : CBType,
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
        let cb = UCallback { command: ManuallyDrop::new(callback) };
        CallbackData { cbtype: CBType::Command, callback: cb, data, hook  }
    }

    /// Creates callback data for a print callback.
    pub (crate)
    fn new_print_data(callback  : Box<PrintCallback>, 
                      data      : UserData,
                      hook      : Hook
                     ) -> Self
    {
        let cb = UCallback { print: ManuallyDrop::new(callback) };
        CallbackData { cbtype: CBType::Print, callback: cb, data, hook }
    }

    /// Creates callback data for a print attrs callback.
    pub (crate)
    fn new_print_attrs_data(callback : Box<PrintAttrsCallback>, 
                            data     : UserData,
                            hook     : Hook
                           ) -> Self
    {
        let cb = UCallback { print_attrs: ManuallyDrop::new(callback) };
        CallbackData { cbtype: CBType::PrintAttrs, callback: cb, data, hook }
    }

    /// Creates callback data for a timer callback.
    pub (crate)
    fn new_timer_data(callback : Box<TimerCallback>, 
                      data     : UserData,
                      hook     : Hook
                     ) -> Self
    {
        let cb = UCallback { timer: ManuallyDrop::new(callback) };
        CallbackData { cbtype: CBType::Timer, callback: cb, data, hook }
    }
    
    /// Creates callback data for a fd callback.
    pub (crate)
    fn new_fd_data(callback : Box<FdCallback>, 
                   data     : UserData,
                   hook     : Hook
                  ) -> Self
    {
        let cb = UCallback { fd: ManuallyDrop::new(callback) };
        CallbackData { cbtype: CBType::Timer, callback: cb, data, hook }
    }

    /// Returns a mutable reference to the Rust-facing `user_data` that was
    /// registered with the callback.    
    #[inline]
    pub (crate)
    fn get_data(&mut self) -> &mut Option<Box<dyn Any>> {
        &mut self.data
    }
    
    /// Returns the `data` (Rust facing user_data) field of the object. 
    /// Ownership of the user_data is transferred to the caller from this 
    /// operation. This is used internally by `Hook::unhook()` to retrieve
    /// callback data when a callback is unregistered. This gives the runtime
    /// the opportunity to free the data by going out of scope, or perform
    /// any custom cleanup. After this function is called, the callback
    /// should be considered finished.
    pub (crate)
    fn take_data(mut self) -> Option<Box<dyn Any>> {
        if self.data.is_some() {
            self.data.take()
        } else {
            None
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
        debug_assert!(CBType::Command == self.cbtype);
        (*self.callback.command)(hc, word, word_eol, ud)
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
        debug_assert!(CBType::Print == self.cbtype);
        (*self.callback.print)(hc, word, ud)
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
        debug_assert!(CBType::PrintAttrs == self.cbtype);
        (*self.callback.print_attrs)(hc, word, attrs, ud)
    }
    
    /// Invokes the callback held in the `callback` field. This is invoked by
    /// c_timer_callback()`.
    #[inline]
    pub (crate)
    unsafe fn timer_cb(&mut self, hc: &Hexchat, ud: &mut UserData) -> i32
    {
        debug_assert!(CBType::Timer == self.cbtype);
        let keep_going = (*self.callback.timer)(hc, ud);
        if keep_going == 0 {
            // Hexchat automatically removes the callback if 0 is returned.
            // This is not good because Hexchat doesn't free the user_data.
            // So we intervene here to ensure the user data gets cleaned up
            // by calling .unhook(). This removes the callback.
            self.hook.unhook();
        }
        // 1 is returned regardless of what the callback itself returns; this
        // is to prevent Hexchat from trying to remove the (already terminated)
        // callback itself, which can crash the application. If the client's
        // Rust-facing callback returned 0 above (keep_going == 0), The
        // callback will have been cleaned up in the conditional above.
        1
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
        debug_assert!(CBType::FD == self.cbtype);
        (*self.callback.fd)(hc, fd, flags, ud)
    }
}

impl Drop for CallbackData {
    /// Causes the destructor for the `self.callback` field to be invoked.
    /// This is called when `CallbackData` is being removed from Hexchat
    /// during an `unhook()` operation.
    ///
    fn drop(&mut self) {
        use CBType::*;
        unsafe {
            // This might be overkill. It might be enough to just pick one of
            // the command types and drop it. But anyway, it's better to err
            // in the direction of safety.
            match self.cbtype {
                Command => {
                    ManuallyDrop::drop(&mut self.callback.command);
                },
                Print => {
                    ManuallyDrop::drop(&mut self.callback.print);
                },
                PrintAttrs => {
                    ManuallyDrop::drop(&mut self.callback.print_attrs);
                },
                Timer => {
                    ManuallyDrop::drop(&mut self.callback.timer);
                },
                FD => {
                    ManuallyDrop::drop(&mut self.callback.fd);
                },
            }
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
                          &mut Option<Box<dyn Any>>
                         ) -> Eat;

/// The Rust-facing function signature corresponding to the C-facing  
/// `C_PrintCallback`. Note that, unlike the C API, the Rust-facing callback
/// signatures include a reference to the Hexchat pointer for 
/// convenience.
pub (crate)
type PrintCallback 
              = dyn FnMut(&Hexchat,
                          &[String],
                          &mut Option<Box<dyn Any>>
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
                          &mut Option<Box<dyn Any>>
                         ) -> Eat;

/// The Rust-facing function signature corresponding to the C-facing  
/// `C_TimerCallback`. Note that, unlike the C API, the Rust-facing callback
/// signatures include a reference to the Hexchat pointer for 
/// convenience.
pub (crate)
type TimerCallback 
              = dyn FnMut(&Hexchat, &mut Option<Box<dyn Any>>) -> i32;

/// The Rust-facing function signature corresponding to the C-facing  
/// `C_FdCallback`. Note that, unlike the C API, the Rust-facing callback
/// signatures include a reference to the Hexchat pointer for 
/// convenience.
pub (crate)
type FdCallback 
              = dyn FnMut(&Hexchat, i32, i32, &mut Option<Box<dyn Any>>) -> Eat;
              
