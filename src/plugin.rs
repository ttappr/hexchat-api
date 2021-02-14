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

#[derive(Clone)]
pub struct Plugin {
    data: Rc<RefCell<PluginData>>,
}

impl Plugin {
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
    fn drop(&mut self) {
        if !self.removed {
            unsafe {
                let hc = &*HEXCHAT;
                (hc.c_plugingui_remove)(hc, self.handle.cast());
            }
        }
    }
}