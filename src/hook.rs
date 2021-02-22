
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
use std::sync::Arc;
use std::sync::Mutex;

use crate::callback_data::*;
use crate::hexchat_entry_points::HEXCHAT;
use crate::user_data::*;

// Hooks are retained for cleanup when deinit is called on plugin unload.
static mut HOOK_LIST: Option<Vec<Hook>> = None;

// TODO - The hook list needs to be synchronized because `main_thread()` will
//        create new hooks from another thread.

use UserData::*;

/// A wrapper for Hexchat callback hooks. These hooks are returned when 
/// registering callbacks and can be used to unregister (unhook) them.
/// `Hook`s can be cloned to share a reference to the same callback hook.
///
#[derive(Clone)]
pub struct Hook {
    hook: Rc<RefCell<*const c_void>>,
}
impl Hook {
    /// Constructor. `hook` is a hook returned by Hexchat when registering a
    /// C-facing callback.
    ///
    pub (crate) fn new() -> Self {
        let hook = Hook { hook: Rc::new(RefCell::new(null::<c_void>())) };
        unsafe {
            if let Some(hook_list) = &mut HOOK_LIST {
                //let hook_list = &mut *hook_list.lock().unwrap();
                hook_list.retain(|h| !h.hook.borrow().is_null());
                hook_list.push(hook.clone());
            } else {
                let mut hook_list = Vec::new();
                hook_list.push(hook.clone());
                //HOOK_LIST = Some(Arc::new(Mutex::new(hook_list)));
                HOOK_LIST = Some(hook_list);
            }
        }
        hook
    }
    
    /// Sets the value of the internal hook pointer.
    pub (crate) fn set(&self, ptr: *const c_void) {
        *self.hook.borrow_mut() = ptr;
    }

    /// Unhooks the related callback from Hexchat. The user_data object is
    /// returned. Subsequent calls to `unhook()` will return `None`. The 
    /// callback that was registered with Hexchat will be unhooked and dropped.
    /// If the user data was one of the shared types, a clone of it is returned.
    /// The boxed type `BoxedData` can't be copied or cloned, so `NoData` will 
    /// be returned in that case. If getting the user data back from a callback
    /// using its hook is needed, consider using one of the shared types 
    /// (`SharedData`, `SyncData`) instead of `BoxedData`.
    /// # Returns
    /// * The user data that was registered with the callback using one of the
    ///   hexchat hook functions. A clone of the data is returned, if possible.
    ///   `NoData` is returned for `NoData` and `BoxedData` types.
    ///
    pub fn unhook(&self) -> UserData {
        unsafe {
            let mut ptr_ref = self.hook.borrow_mut();
            if !ptr_ref.is_null() {
                let hc = &*HEXCHAT;
                let cd = (hc.c_unhook)(hc, *ptr_ref);
                if !cd.is_null() {
                    // TODO - Find out why this is necessary. cd should never
                    //        be null when we're here. Why is c_unhook() 
                    //        returning null pointers??
                    let cd = &mut (*(cd as *mut CallbackData));
                    let cd = Box::from_raw(cd);
                    *ptr_ref = null::<c_void>();
                    cd.get_data()
                } else {
                    NoData
                }
            } else {
                NoData
            }
        }
    }
    
    /// Called when a plugin is unloaded by Hexchat. This happens when the user
    /// opens the "Plugins and Scripts" dialog and unloads/reloads the plugin,
    /// or the user issues one of the slash "/" commands to perform the same
    /// operation. This function iterates over each hook, calling their 
    /// `unhook()` method which grabs ownership of the `CallbackData` objects
    /// and drops them as they go out of scope ensuring their destructors
    /// are called.
    ///
    pub (crate) fn deinit() {
        unsafe {
            if let Some(hook_list) = &HOOK_LIST {
                //let hook_list = &mut *hook_list.lock().unwrap();
                for hook in hook_list {
                    hook.unhook();
                }
                HOOK_LIST = None;
            }
        }
    }    
}
