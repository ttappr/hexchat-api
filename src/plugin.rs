#![allow(dead_code)]

use std::ffi::{CString, c_void};
use crate::{str2cstring, HEXCHAT};
use crate::cbuf;
use std::ptr;
use std::cell::RefCell;
use std::rc::Rc;

// TODO - Currently there's no use for retaining all the data except for the
//        handle. Consider removing the CString fields.
struct PluginData {
    file_name   : CString,
    plugin_name : CString,
    description : CString,
    version     : CString,
    handle      : *const c_void,
    removed     : bool,
}

/// Represents a created plugin entry. Plugins that embed other language
/// interpreters and load plugins written in those languages can have Hexchat
/// look as if the loaded scripts are actual plugins. By creating a `Plugin`
/// object for such a script, an entry is made in Hexchat's list of loaded
/// plugins. When one of these scripts is unloaded, the fictitious plugin entry
/// can be removed from Hexchat by dropping the associated `Plugin` object.
///
#[derive(Clone)]
pub struct Plugin {
    data: Rc<RefCell<PluginData>>,
}

impl Plugin {
    /// Creates a new plugin entry in Hexchat.
    /// # Arguments
    /// `file_name`     - The name of the script representing a "plugin".
    /// `plugin_name`   - The name of the plugin script.
    /// `description`   - The plugin script's description.
    /// `version`       - A version string for the plugin script.
    ///
    pub fn new(file_name    : &str,
               plugin_name  : &str,
               description  : &str,
               version      : &str
              ) -> Plugin
    {
        unsafe {
            let hc   = &*HEXCHAT;
            let null = ptr::null::<c_void>();
            let handle = (hc.c_plugingui_add)(hc,
                                              cbuf!(file_name),
                                              cbuf!(plugin_name),
                                              cbuf!(description),
                                              cbuf!(version),
                                              null.cast());
            let pd = PluginData {
                file_name   : str2cstring(file_name),
                plugin_name : str2cstring(plugin_name),
                description : str2cstring(description),
                version     : str2cstring(version),
                handle      : handle.cast(),
                removed     : false,
            };
            Plugin { data: Rc::new(RefCell::new(pd)) }
        }
    }
    /// Removes the plugin entry for the plugin script. This can be used to
    /// remove a plugin entry, or simply dropping the `Plugin` object will
    /// cause removal to happen automatically.
    ///
    pub fn remove(&self) {
        let cell = &*self.data;
        let data = &mut *cell.borrow_mut();
        if !data.removed {
            unsafe {
                let hc = &*HEXCHAT;
                (hc.c_plugingui_remove)(hc, data.handle.cast());
            }
            data.removed = true;
        }
    }
}

impl Drop for PluginData {
    /// Removes the entry in Hexchat's plugins list for the `Plugin`.
    fn drop(&mut self) {
        if !self.removed {
            unsafe {
                let hc = &*HEXCHAT;
                (hc.c_plugingui_remove)(hc, self.handle.cast());
            }
        }
    }
}
