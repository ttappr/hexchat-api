
#![allow(dead_code)]

use libc::{c_char, c_void};
use std::ffi::{CString, CStr};
use std::cell::RefCell;
use std::rc::Rc;
use crate::hexchat::{Hexchat, hexchat_context};
use crate::hexchat_entry_points::HEXCHAT;
use crate::list_iterator::{ListIterator, ListError, FieldValue};
use crate::utils::*;

pub struct Context {
    data    : Rc<RefCell<ContextData>>,
}

impl Context {
    pub
    fn find(network: &str, channel: &str) -> Self {
        let network = str2cstring(network);
        let channel = str2cstring(channel);
        let hc = unsafe { &*HEXCHAT };
        let context_ptr;
        unsafe {
            context_ptr = (hc.c_find_context)(hc,
                                              network.as_ptr(),
                                              channel.as_ptr());
        }
        Context {
            data: Rc::new(RefCell::new(
                ContextData {
                    hc,
                    context_ptr,
                    network,
                    channel ,
                }))}
    }

    pub fn get() -> Self {
        let network = str2cstring("network");
        let channel = str2cstring("channel");
        unsafe {
            let hc      = &*HEXCHAT;
            let ctx_ptr = (hc.c_get_context)(hc);
            let network = (hc.c_get_info)(hc, network.as_ptr());
            let channel = (hc.c_get_info)(hc, channel.as_ptr());
            Context {
                data: Rc::new(RefCell::new(
                    ContextData {
                        hc,
                        context_ptr : ctx_ptr,
                        network     : pchar2cstring(network),
                        channel     : pchar2cstring(channel),
                    }))
            }
        }
    }

    pub fn set(&self) -> i32 {
        let data = self.data.borrow_mut();
        unsafe {
            (data.hc.c_set_context)(data.hc, data.context_ptr)
        }
    }
    pub fn print(&self, message: &str) {
        let data = self.data.borrow_mut();
        let msg  = str2cstring(message);
        unsafe {
            let prior  = (data.hc.c_get_context)(data.hc);
            (data.hc.c_set_context)(data.hc, data.context_ptr);
            (data.hc.c_print)(data.hc, msg.as_ptr());
            (data.hc.c_set_context)(data.hc, prior);
        }
    }
    pub fn emit_print(&self, event_name: &str, var_args: &[&str]) {
        let data = self.data.borrow_mut();
        unsafe {
            let prior  = (data.hc.c_get_context)(data.hc);
            (data.hc.c_set_context)(data.hc, data.context_ptr);
            self.emit_print(event_name, var_args);
            (data.hc.c_set_context)(data.hc, prior);
        }
    }
    pub fn command(&self, command: &str) {
        let data = self.data.borrow_mut();
        let cmd  = str2cstring(command);
        unsafe {
            let prior  = (data.hc.c_get_context)(data.hc);
            (data.hc.c_set_context)(data.hc, data.context_ptr);
            (data.hc.c_command)(data.hc, cmd.as_ptr());
            (data.hc.c_set_context)(data.hc, prior);
        }
    }
    pub fn get_info(&self, list: &str) -> String {
        let data = self.data.borrow_mut();
        let list = str2cstring(list);
        unsafe {
            let prior  = (data.hc.c_get_context)(data.hc);
            (data.hc.c_set_context)(data.hc, data.context_ptr);
            let result = (data.hc.c_get_info)(data.hc, list.as_ptr());
            (data.hc.c_set_context)(data.hc, prior);
            p_char_to_string(result)
        }
    }
    pub fn get_listiter(&self, list: &str) -> Result<ListIterator, ListError> {
        let data = self.data.borrow_mut();
        unsafe {
            let prior  = (data.hc.c_get_context)(data.hc);
            (data.hc.c_set_context)(data.hc, data.context_ptr);
            let iter = ListIterator::new(list);
            (data.hc.c_set_context)(data.hc, prior);
            iter
        }
    }
}

struct ContextData {
    hc          : &'static Hexchat,
    context_ptr : *const hexchat_context,
    network     : CString,
    channel     : CString,
}

#[inline]
fn p_char_to_string(p_char: *const c_char) -> String {
    if p_char.is_null() {
        String::new()
    } else {
        unsafe {
            CStr::from_ptr(p_char).to_string_lossy().into_owned()
        }
    }
}