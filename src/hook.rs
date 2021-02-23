
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
use std::sync::RwLock;

use crate::callback_data::*;
use crate::hexchat_entry_points::HEXCHAT;
use crate::user_data::*;

/// A synchronized global list of the hooks. This gets initialized when a 
/// plugin is loaded from within the `lib_hexchat_plugin_init()` function
/// before the plugin author's registered init function is invoked.
///
static mut HOOK_LIST: Option<RwLock<Vec<Hook>>> = None;

use UserData::*;

/// A wrapper for Hexchat callback hooks. These hooks are returned when 
/// registering callbacks and can be used to unregister (unhook) them.
/// `Hook`s can be cloned to share a reference to the same callback hook.
///
#[derive(Clone)]
pub struct Hook {
    hook: Rc<RefCell<*const c_void>>,
}

unsafe impl Send for Hook {}
unsafe impl Sync for Hook {}

impl Hook {
    /// Constructor. `hook` is a hook returned by Hexchat when registering a
    /// C-facing callback.
    ///
    pub (crate) fn new() -> Self {

        let hook = Hook { 
            hook: Rc::new(RefCell::new(null::<c_void>())) 
        };
                   
        if let Some(hook_list_rwlock) = unsafe { &HOOK_LIST } {
            // Acquire global hook list write lock.
            let wlock     = hook_list_rwlock.write();
            let hook_list = &mut *wlock.unwrap();
            
            // Clean up dead hooks.
            hook_list.retain(|h| !h.hook.borrow().is_null());
            
            // Store newly created hook in global list.
            hook_list.push(hook.clone());
        }

        hook
    }
    
    /// Sets the value of the internal hook pointer. This is used by the hooking
    /// functions in hexchat.rs.
    ///
    pub (crate) fn set(&self, ptr: *const c_void) {
        if let Some(hl_rwlock) = unsafe { &HOOK_LIST } {
            // Lock the global list, and set the internal pointer.
            let rlock = hl_rwlock.read();
            *self.hook.borrow_mut() = ptr;
        }
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
            if let Some(hl_rwlock) = &HOOK_LIST {
                let rlock = hl_rwlock.read();
                
                // Determine if the Hook is still alive (non-null ptr).
                let mut ptr_ref = self.hook.borrow_mut();
                
                if !ptr_ref.is_null() {
                    // Unhook the callback and retrieve the user data pointer.
                    let hc = &*HEXCHAT;
                    let cd = (hc.c_unhook)(hc, *ptr_ref);
                    
                    // TODO - Find out why c_unhook() sometimes yields null.
                    if !cd.is_null() {
                        // Reconstruct the CallbackData Box.
                        let cd = &mut (*(cd as *mut CallbackData));
                        let cd = Box::from_raw(cd);
                        
                        // Null the hook pointer.
                        *ptr_ref = null::<c_void>();
                        
                        // Give back the UserData registered with the callback.
                        return cd.get_data();
                        
                    }
                }
            }
            NoData
        }
    }

    /// Called automatically within `lib_hexchat_plugin_init()` when a plugin is 
    /// loaded. This initializes the synchronized global static hook list.
    ///
    pub (crate) fn init() {
        unsafe {
            HOOK_LIST = Some(RwLock::new(Vec::new()));
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
        if let Some(hl_rwlock) = unsafe { &HOOK_LIST } {
            let rlock = hl_rwlock.read();
            let hook_list = &*rlock.unwrap();
            for hook in hook_list {
                hook.unhook();
            }
        }
        unsafe {
            // This causes the `RwLock` and hook vector to be dropped. 
            // plugin authors need to ensure that no threads are running when
            // their plugins are unloading - or one may try to access the lock
            // and hook vector after they've been destroyed.
            let _ = HOOK_LIST.take();
        }
    }
}



