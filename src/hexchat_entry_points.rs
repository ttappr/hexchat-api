
#![allow(dead_code)]

//! This module holds the DLL entry points for this plugin library.
//! These exported functions are what Hexchat links to directly when the
//! plugin is loaded.
//! 
//! These functions register the plugin info with Hexchat and set up to "catch"
//! any panics that Rust-side callbacks might "raise" during execution.
//! Panics are displayed in the currently active Hexchat window/context.
//! The debug build of the library will include a stack trace in the error 
//! message.

#[cfg(debug_assertions)]
use backtrace::Backtrace;
use libc::{c_int, c_char, c_void};
use std::ffi::CString;
use std::marker::PhantomPinned;
use std::panic;
use std::panic::{catch_unwind, UnwindSafe};
use std::pin::Pin;
use std::ptr::null;
use std::ptr::NonNull;

//use crate::{plugin_get_info, plugin_init, plugin_deinit};
use crate::hexchat::Hexchat;
use crate::utils::*;

/// The signatures for the required functions a plugin needs to implement to
/// create a loadable Hexchat plugin.
pub type InitFn   = dyn FnOnce(&'static Hexchat) -> i32 + UnwindSafe;
pub type DeinitFn = dyn FnOnce(&'static Hexchat) -> i32 + UnwindSafe;
pub type InfoFn   = dyn FnOnce()                 -> Pin<Box<PluginInfo>>
                                                        + UnwindSafe;

/// Holds persistent client plugin info strings.
static mut PLUGIN_INFO: Option<Pin<Box<PluginInfo>>> = None;

/// The global Hexchat pointer obtained from `hexchat_plugin_init()`.
pub (crate)
static mut HEXCHAT: *const Hexchat = null::<Hexchat>();

/// Plugins using this API can use this macro to create the necessary
/// DLL entry points for Hexchat to find while the DLL is being loaded.
/// Normally implemented functions that have the required signatures can
/// be passed to the macro like so:
///
/// ```dll_entry_points!( my_info_func, my_init_func, my_deinit_func )```
///
/// # The signatures for the functions are:
/// * my_info_func  ()                 -> Pin<Box<PluginInfo>>;
/// * my_init_func  (&'static Hexchat) -> i32;
/// * my_deinit_func(&'static Hexchat) -> i32;
///
/// The info function creates an instance of `PluginInfo` by calling its
/// constructor with the information about the plugin as parameters. The
/// constructor returns a `Pin<Box<PluginInfo>>` instance - this can be
/// returned as-is from `my_info_func()`.
///
/// The init function is usually where all the commands are registered using
/// the hook commands provided by the `&Hexchat` reference provided as
/// a paramter to the it function. The init function needs to return either 0
/// (good) or 1 (error).
///
/// The deinit function gets called when the plugin is unloaded. It also returns
/// 0 (good) or 1 (error). Any cleanup actions needed to be done ccan be done
/// here. When  DLL is unloaded by Hexchat, all its hooked commands are unhooked
/// automatically - so that doesn't need to be done by this function.
///
#[macro_export]
macro_rules! dll_entry_points {
 
    ( $info:ident, $init:ident, $deinit:ident ) => {
        #[no_mangle]
        pub extern "C"    
        fn hexchat_plugin_get_info(name     : *mut *const i8,
                                   desc     : *mut *const i8,
                                   version  : *mut *const i8,
                                   reserved : *mut *const i8) 
        {
            hexchat_api::lib_get_info(name,    
                                      desc,
                                      version,
                                      Box::new($info));
        }
        #[no_mangle]
        pub extern "C"
        fn hexchat_plugin_init(hexchat   : &'static Hexchat,
                               name      : *mut *const i8,
                               desc      : *mut *const i8,
                               version   : *mut *const i8
                              ) -> i32
        {
            hexchat_api::lib_hexchat_plugin_init(hexchat, 
                                                 name,
                                                 desc,   
                                                 version,
                                                 Box::new($init),
                                                 Box::new($info))
        }
        #[no_mangle]
        pub extern "C"
        fn hexchat_plugin_deinit(hexchat : &'static Hexchat) -> i32
        {
            hexchat_api::lib_hexchat_plugin_deinit(hexchat, Box::new($deinit))
        }
    }
}

/// Holds client plugin information strings.
pub struct PluginInfo {
    name         : CString,
    version      : CString,
    description  : CString,
    pname        : NonNull<CString>,
    pversion     : NonNull<CString>,
    pdescription : NonNull<CString>,
    _pin         : PhantomPinned,
}
impl PluginInfo {
    /// Constructor. The plugin information provided in the parameters is used
    /// to create persistent pinned buffers that are guaranteed to be valid
    /// for Hexchat to read from while the plugin is loading.
    ///
    /// # Arguments
    /// * `name`        - The name of the plugin.
    /// * `version`     - The plugin's version number.
    /// * `description` - The plugin's description.
    ///
    /// # Returns
    /// A `PluginInfo` object initialized from the parameter data.
    ///
    pub fn new(name: &str, version: &str, description: &str) -> Pin<Box<Self>>
    {
        let pi = PluginInfo {
            name         : str2cstring(name),
            version      : str2cstring(version),
            description  : str2cstring(description),
            pname        : NonNull::dangling(),
            pversion     : NonNull::dangling(),
            pdescription : NonNull::dangling(),
            _pin         : PhantomPinned,
        };
        let mut boxed    = Box::pin(pi);
        let sname        = NonNull::from(&boxed.name);
        let sversion     = NonNull::from(&boxed.version);
        let sdescription = NonNull::from(&boxed.description);
        
        unsafe {
            let mut_ref: Pin<&mut Self> = Pin::as_mut(&mut boxed);
            let unchecked = Pin::get_unchecked_mut(mut_ref); 
            unchecked.pname        = sname;
            unchecked.pversion     = sversion;
            unchecked.pdescription = sdescription;
        }
        boxed
    }
}

/// An exported function that Hexchat calls when loading the plugin.
/// This function calls the client plugin's `plugin_get_info()` indirectly to
/// obtain the persistent plugin info strings that it sets the paramters to.
pub fn lib_hexchat_plugin_get_info(name      : *mut *const i8,
                                   desc      : *mut *const i8,
                                   version   : *mut *const i8,
                                   reserved  : *mut *const i8,
                                   callback  : Box<InfoFn>)
{
    lib_get_info(name, desc, version, callback);
}

/// An exported function called by Hexchat when the plugin is loaded.
/// This function calls the client plugin's `plugin_init()`.
pub fn lib_hexchat_plugin_init(hexchat   : &'static Hexchat,
                               name      : *mut *const c_char,
                               desc      : *mut *const c_char,
                               version   : *mut *const c_char,
                               init_cb   : Box<InitFn>,
                               info_cb   : Box<InfoFn>
                              ) -> i32
{
    // Store the global Hexchat pointer.
    unsafe { HEXCHAT = hexchat; }

    set_panic_hook(hexchat);

    lib_get_info(name, desc, version, info_cb);

    // Invoke client lib's init function.
    catch_unwind(|| { init_cb(hexchat) }).unwrap_or(0)
}

/// An exported function called by Hexchat when the plugin is unloaded.
/// This function calls the client plugin's `plugin_deinit()`.
pub fn lib_hexchat_plugin_deinit(hexchat  : &'static Hexchat, 
                                 callback : Box<DeinitFn>
                                ) -> i32
{
    catch_unwind(|| { callback(hexchat) }).unwrap_or(0)
}


/// Sets the parameter pointers to plugin info strings.
#[inline]
pub fn lib_get_info(name     : *mut *const c_char,
                    desc     : *mut *const c_char,
                    vers     : *mut *const c_char,
                    callback : Box<InfoFn>)
{
    unsafe {
        if PLUGIN_INFO.is_none() {
            let pi = callback();
            PLUGIN_INFO = Some(pi);
        }
        if let Some(info) = &PLUGIN_INFO {
            *name = info.pname.as_ref().as_ptr();
            *vers = info.pversion.as_ref().as_ptr();
            *desc = info.description.as_ref().as_ptr();
        }
    }
}

/// Sets the panic hook so panic info will be printed to the active Hexchat 
/// window. The debug build includes a stack trace using  
/// [Backtrace](https://crates.io/crates/backtrace)
fn set_panic_hook(hexchat: &'static Hexchat) 
{
    panic::set_hook(Box::new(move |panic_info| {
        #[cfg(debug_assertions)]
        let mut loc = String::new();
        
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            hexchat.print(&format!("\x0304<<Panicked!>>\t{:?}", s));
        } else {
            hexchat.print(&format!("\x0304<<Panicked!>>\t{:?}", panic_info));
        }
        if let Some(location) = panic_info.location() {
            hexchat.print(
                &format!("\x0313Panic occured in file '{}' at line {}.",
                         location.file(),
                         location.line()
                        ));
                        
            #[cfg(debug_assertions)]
            {loc = format!("{}:{}", location.file(), location.line());}
        }
        // For the debug build, include a stack trace.
        #[cfg(debug_assertions)]
        {
            let mut trace = vec![];
            let mut begin = 0;
            let mut end   = 0;
            let     bt    = Backtrace::new();
            let     btstr = format!("{:?}", bt);
            
            for line in btstr.lines() {
                let line  = String::from(line);
                if begin == 0 && !loc.is_empty() && line.contains(&loc) {
                    // Underlined and magenta.
                    trace.push(format!("\x1F\x0313{}", line));
                    begin = end;
                } else {
                    trace.push(format!("\x0304{}", line));
                }
                end += 1;
            }
            // Start the trace where the panic actually occurred.
            begin = if begin == 0 { 0 } else { begin - 1 };
            hexchat.print(&trace[begin..end].join("\n"));
        }
    }));
}
