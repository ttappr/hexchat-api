
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
use std::panic;
use std::panic::{catch_unwind, UnwindSafe};
use std::ptr::null;

//use crate::{plugin_get_info, plugin_init, plugin_deinit};
use crate::hexchat::Hexchat;
use crate::utils::*;

/// Holds persistent client plugin info strings.
static mut PLUGIN_INFO: Option<PluginInfo> = None;

/// The global Hexchat pointer obtained from `hexchat_plugin_init()`.
pub (crate)
static mut HEXCHAT: *const Hexchat = null::<Hexchat>();

#[macro_export]
macro_rules! blah {
 
    ( $info:ident, $init:ident, $deinit:ident ) => {
        #[no_mangle]
        pub extern "C"    
        fn hexchat_plugin_get_info(name     : *mut *const i8,
                                   desc     : *mut *const i8,
                                   version  : *mut *const i8,
                                   reserved : *mut *const i8) 
        {
            println!("hexchat_plugin_get_info()");
            hexchat_api::lib_get_info(name, desc, version, Box::new($info));
        }
        #[no_mangle]
        pub extern "C"
        fn hexchat_plugin_init(hexchat   : &'static Hexchat,
                               name      : *mut *const i8,
                               desc      : *mut *const i8,
                               version   : *mut *const i8
                              ) -> i32
        {
            println!("hexchat_plugin_init()");
            hexchat_api::lib_get_info(name, desc, version, Box::new($info));
            hexchat_api::lib_hexchat_plugin_init(hexchat, 
                                                 name,
                                                 desc,   
                                                 version,
                                                 Box::new($init),
                                                 Box::new($info));
            0
        }
        #[no_mangle]
        pub extern "C"
        fn hexchat_plugin_deinit(hexchat : &'static Hexchat) -> i32
        {
            println!("hexchat_plugin_deinit()");
            hexchat_api::lib_hexchat_plugin_deinit(hexchat, Box::new($deinit));
            0
        }

    }
}


pub fn foo(a: &str) {
}

/// Holds client plugin information strings.
pub struct PluginInfo {
    name        : CString,
    version     : CString,
    description : CString,
}
impl PluginInfo {
    /// Constructor.
    ///
    /// # Arguments
    /// * `name`        - The name of the plugin.
    /// * `version`     - The plugin's version number.
    /// * `description` - The plugin's description.
    ///
    /// # Returns
    /// A `PluginInfo` object initialized from the parameter data.
    ///
    pub fn new(name: &str, version: &str, description: &str) -> Self 
    {
        PluginInfo {
            name        : str2cstring(name),
            version     : str2cstring(version),
            description : str2cstring(description),
        }
    }
}

pub type InitFn   = dyn FnOnce(&'static Hexchat) -> i32 + UnwindSafe;
pub type DeinitFn = dyn FnOnce(&'static Hexchat) -> i32 + UnwindSafe;
pub type InfoFn   = dyn FnOnce()                 -> PluginInfo + UnwindSafe;


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
            *name = info.name.as_ptr();
            *vers = info.version.as_ptr();
            *desc = info.description.as_ptr();
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
