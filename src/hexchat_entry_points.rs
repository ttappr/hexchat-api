
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
use libc::c_char;
use std::ffi::CString;
use std::marker::PhantomPinned;
use std::panic;
use std::panic::{catch_unwind, UnwindSafe};
use std::pin::Pin;
use std::ptr::null;
use std::ptr::NonNull;

//use crate::{plugin_get_info, plugin_init, plugin_deinit};
use crate::hexchat::Hexchat;
use crate::hook::*;
use crate::utils::*;
use crate::thread_facilities::*;

/// The signature for the init function that plugin authors need to register
/// using `dll_entry_points!()`.
pub type InitFn   = dyn FnOnce(&'static Hexchat) -> i32 + UnwindSafe;

/// The signature for the deinit function plugin authors need to register using
/// `dll_entry_points!()`.
pub type DeinitFn = dyn FnOnce(&'static Hexchat) -> i32 + UnwindSafe;

/// The signature of the info function plugin authors need to register using
/// `dll_entry_points!()`.
pub type InfoFn   = dyn FnOnce() -> PluginInfo + UnwindSafe;

/// Holds persistent client plugin info strings.
static mut PLUGIN_INFO: Option<PluginInfo> = None;

/// The global Hexchat pointer obtained from `hexchat_plugin_init()`.
pub(crate) static mut PHEXCHAT: *const Hexchat = null::<Hexchat>();

/// `dll_entry_points()` makes it very easy to set up your plugin's DLL
/// interface required by the Hexchat loader. This macro generates the necessary
/// DLL entry points that Hexchat looks for when a DLL is being loaded.
/// Normal Rust functions having the required signatures can be passed to the
/// macro like so:
///
/// ```dll_entry_points!( my_info_func, my_init_func, my_deinit_func )```
///
/// That's it. You don't need to worry about how to export your Rust functions
/// to interface with the C environment of Hexchat. This macro does it all for
/// you.
///
/// # The signatures for the functions are:
///
/// * `my_info_func  ()                 -> PluginInfo;`
/// * `my_init_func  (&'static Hexchat) -> i32;`
/// * `my_deinit_func(&'static Hexchat) -> i32;`
///
/// The **info function** should create an instance of `PluginInfo` by calling
/// its constructor with information about the plugin as parameters.
///
/// The **init function** is typically where you'll want to register your
/// plugin's commands. Hook commands are provided by the `&Hexchat` reference
/// provided as a paramter when your init function is called by Hexchat. The
/// init function needs to return either 0 (good) or 1 (error).
///
/// The **deinit function** gets called when your plugin is unloaded. Return a
/// 0 (good) or 1 (error). Any cleanup actions needed to be done can be done
/// here. However, when your  DLL is unloaded by Hexchat, all its hooked
/// commands are unhooked automatically - so you don't need to worry about
/// managing the `Hook` objects returned by the hook commands unless you're
/// plugin needs to for some reason. If your plugin creates any static
/// variables, This is the place to drop their values, for example:
/// `MY_STATIC_VAR = None;`
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
    };
}

/// Holds client plugin information strings.
struct PluginInfoData {
    name         : CString,
    version      : CString,
    description  : CString,
    pname        : NonNull<CString>,
    pversion     : NonNull<CString>,
    pdescription : NonNull<CString>,
    _pin         : PhantomPinned,
}

/// Hexchat addons need to return an instance of this struct from their
/// `plugin_info()` function, which gets called when Hexchat loads the addons.
/// The `PluginInfo` object holds pinned internal buffers that Hexchat can
/// read from at its leisure.
///
pub struct PluginInfo {
    data: Pin<Box<PluginInfoData>>,
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
    pub fn new(name: &str, version: &str, description: &str) -> PluginInfo
    {
        let pi = PluginInfoData {
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
            let mut_ref: Pin<&mut PluginInfoData> = Pin::as_mut(&mut boxed);
            let unchecked = Pin::get_unchecked_mut(mut_ref); 
            unchecked.pname        = sname;
            unchecked.pversion     = sversion;
            unchecked.pdescription = sdescription;
        }
        PluginInfo { data: boxed }
    }
}

/// Called indirectly when a plugin is loaded to get info about it. The
/// plugin author shouldn't invoke this fuction - it's only public because
/// the `dll_entry_points()` macro generates code that calls this.
/// This function calls the client plugin's `plugin_get_info()` indirectly to
/// obtain the persistent plugin info strings that it sets the paramters to.
///
#[doc(hidden)]
pub fn lib_hexchat_plugin_get_info(name      : *mut *const i8,
                                   desc      : *mut *const i8,
                                   version   : *mut *const i8,
                                   _reserved : *mut *const i8,
                                   callback  : Box<InfoFn>)
{
    lib_get_info(name, desc, version, callback);
}

/// Called indirectly while a plugin is being loaded. The
/// plugin author shouldn't invoke this fuction - it's only public because
/// the `dll_entry_points()` macro generates code that calls this.
///
#[doc(hidden)]
pub fn lib_hexchat_plugin_init(hexchat   : &'static Hexchat,
                               name      : *mut *const c_char,
                               desc      : *mut *const c_char,
                               version   : *mut *const c_char,
                               init_cb   : Box<InitFn>,
                               info_cb   : Box<InfoFn>) 
    -> i32
{
    // Store the global Hexchat pointer.
    unsafe { PHEXCHAT = hexchat; }

    set_panic_hook(hexchat);

    lib_get_info(name, desc, version, info_cb);

    // Invoke client lib's init function.
    catch_unwind(|| { 
        Hook::init();
        main_thread_init();
        init_cb(hexchat) 
    }).unwrap_or(0)
}

/// Invoked indirectly while a plugin is being unloaded. This function will
/// call the deinitialization function that was registered using the
/// `dll_entry_points()` macro. It will also unhook all the callbacks
/// currently registered forcing them, and their closure state, to drop and
/// thus clean up. Plugin authors should not call this - it's only public 
/// because `dll_entry_points()` generates code that needs this.
///
#[doc(hidden)]
pub fn lib_hexchat_plugin_deinit(hexchat  : &'static Hexchat, 
                                 callback : Box<DeinitFn>) 
    -> i32
{
    let result = catch_unwind(|| {
        // Call user's deinit().
        let retval = callback(hexchat);
        
        main_thread_deinit();
        
        // Cause the callback_data objects to drop and clean up.
        Hook::deinit();   
        
        // Destruct the info struct.
        unsafe { PLUGIN_INFO = None; }
        
        retval     
    }).unwrap_or(0);
    // Final clean up on unload - drop the hook closure.
    let _ = panic::take_hook();
    result
}


/// This function sets Hexchat's character pointer pointer's to point at the
/// pinned buffers holding info about a plugin. Not to be called by plugin
/// authors - it's only public because `dll_entry_points()` generates code
/// that calls this.
///
#[inline]
#[doc(hidden)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
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
            *name = info.data.pname.as_ref().as_ptr();
            *vers = info.data.pversion.as_ref().as_ptr();
            *desc = info.data.description.as_ref().as_ptr();
        }
    }
}

/// Sets the panic hook so panic info will be printed to the active Hexchat 
/// window. The debug build includes a stack trace using  
/// [Backtrace](https://crates.io/crates/backtrace)
fn set_panic_hook(hexchat: &'static Hexchat) {
    panic::set_hook(Box::new(move |panic_info| {
        #[cfg(debug_assertions)]
        let mut loc = String::new();
        let plugin_name;
        unsafe {
            if let Some(plugin_info) = &PLUGIN_INFO {
                plugin_name = plugin_info.data.name.to_str().unwrap();
            } else {
                plugin_name = "a Rust plugin";
            }
        }
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            hexchat.print(&format!("\x0304<<Panicked!>>\t{:?}", s));
        } else {
            hexchat.print(&format!("\x0304<<Panicked!>>\t{:?}", panic_info));
        }
        if let Some(location) = panic_info.location() {
            hexchat.print(
                &format!("\x0313Panic occured in {} in file '{}' at line {:?}.",
                         plugin_name,
                         location.file(),
                         location.line()));
                        
            #[cfg(debug_assertions)]
            { loc = format!("{}:{:?}", location.file(), location.line()); }
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
