
//! This object wraps hook pointers returned when callbacks are registered
//! with Hexchat. The Rust-style hook can be used to unhook commands directly
//! via its `unhook()` function. This object protects against attempts to
//! unhook the same callback more than once, which can crash Hexchat.
//! 
//! The `unhook()` function returns the user_data that was registered with
//! the associated callback, passing ownership to the caller. Invoking
//! `unhook()` more than once returns `None`.
//! 
//! The hooks can be cloned. Internally, clones safely share the same hook
//! pointer. When hooks go out of scope, they do not remove their associated
//! commands. Hooks can be ignored by the plugin if there is no need to 
//! unhook commands. The most relevant use of a hook could be to cancel
//! timer callbacks.

use libc::c_void;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::ptr::null;

use crate::callback_data::*;
use crate::hexchat_entry_points::HEXCHAT;

/// A wrapper for Hexchat callback hooks. These hooks are returned when 
/// registering callbacks and can be used to unregister (unhook) them.
#[derive(Clone)]
pub struct Hook {
    hook: Rc<RefCell<*const c_void>>,
}
impl Hook {
    /// Constructor. `hook` is a hook returned by Hexchat when registering a
    /// C-facing callback.
    pub (crate)
    fn new() -> Self {
        Hook { hook: Rc::new(RefCell::new(null::<c_void>())) }
    }
    
    /// Sets the value of the internal hook pointer.
    pub (crate)
    fn set(&self, ptr: *const c_void) {
        *self.hook.borrow_mut() = ptr;
    }

    /// Unhooks the related callback from Hexchat. The user_data object is
    /// returned, passing ownership to the caller. Subsequent calls to 
    /// `unhook()` will return `None`.
    pub fn unhook(&self) -> Option<Box<dyn Any>> {
        unsafe {
            let mut ptr_ref = self.hook.borrow_mut();
            if !ptr_ref.is_null() {
                let hc = &*HEXCHAT;
                let cd = (hc.c_unhook)(hc, *ptr_ref);
                let cd = &mut (*(cd as *mut CallbackData));
                let cd = Box::from_raw(cd);
                *ptr_ref = null::<c_void>();
                cd.take_data()
            } else {
                None
            }
        }
    }
}
