
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
use std::sync::RwLock;
use lazy_static::lazy_static;

use crate::callback_data::*;
use crate::hexchat_entry_points::HEXCHAT;
use crate::user_data::*;

/// A synchronized global list of the hooks. This is a semi dangerous approach,
/// but should be okay by making sure `Hook::init()` is called in the plugin
/// init DLL entry function - before any code tries to hook any commands.
/// Also, when the deinit DLL entry function is called, the only thing that
/// can crash the software would be if the plugin author left any threads
/// running. So it has to be stipulated that plugin authors ensure no threads
/// are running when their plugins are unloaded - they need to join them in
/// their deinit functions registered using `dll_entry_points()`. `lazy_static`
/// might seem applicable here, but it leaves behind some system resources that
/// can't be reliably free'd when a plugin is unloaded.
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
    let hook = Hook { hook: Rc::new(RefCell::new(null::<c_void>())) };
        if let Some(hook_list_rwlock) = unsafe { &HOOK_LIST } {
            let wlock     = hook_list_rwlock.write();
            let hook_list = &mut *wlock.unwrap();
            hook_list.retain(|h| !h.hook.borrow().is_null());
            hook_list.push(hook.clone());
        }
        hook
    }
    
    pub (crate) fn init() {
        unsafe {
            HOOK_LIST = Some(RwLock::new(Vec::new()));
        }
    }
    
    /// Sets the value of the internal hook pointer.
    pub (crate) fn set(&self, ptr: *const c_void) {
        if let Some(hl_rwlock) = unsafe { &HOOK_LIST } {
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
        if let Some(hl_rwlock) = unsafe { &HOOK_LIST } {
            let rlock = hl_rwlock.read();
            let hook_list = &*rlock.unwrap();
            for hook in hook_list {
                hook.unhook();
            }
        }
        unsafe {
            let _ = HOOK_LIST.take();
        }
    }
}



