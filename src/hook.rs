
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

// Hooks are retained for cleanup when deinit is called on plugin unload.
static mut HOOK_LIST: Option<Vec<Hook>> = None;


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
        let hook = Hook { hook: Rc::new(RefCell::new(null::<c_void>())) };
        unsafe {
            if let Some(hook_list) = &mut HOOK_LIST {
                hook_list.retain(|h| !h.hook.borrow().is_null());
                hook_list.push(hook.clone());
            } else {
                let mut hook_list = Vec::new();
                hook_list.push(hook.clone());
                HOOK_LIST = Some(hook_list);
            }
        }
        hook
    }
    
    /// Sets the value of the internal hook pointer.
    pub (crate)
    fn set(&self, ptr: *const c_void) {
        *self.hook.borrow_mut() = ptr;
    }

    /// Unhooks the related callback from Hexchat. The user_data object is
    /// returned, passing ownership to the caller. Subsequent calls to 
    /// `unhook()` will return `None`. The callback that was registered with
    /// Hexchat will be unhooked and dropped.
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
    
    /// Called when a plugin is unloaded by Hexchat. This happens when the user
    /// opens the "Plugins and Scripts" dialog and unloads/reloads the plugin,
    /// or the user issues one of the slash "/" commands to perform the same
    /// operation. This function iterates over each hook, calling their 
    /// `unhook()` method which grabs the callback data into ownership on the 
    /// stack, which then goes out of scope, thus cleaning up each 
    /// `CallbackData`  object. This is relevant for closures, as they have
    /// state associated with them that gets freed.
    pub (crate) fn deinit() {
        unsafe {
            if let Some(hook_list) = &HOOK_LIST {
                for hook in hook_list {
                    hook.unhook();
                }
                HOOK_LIST = None;
            }
        }
    }    
}








