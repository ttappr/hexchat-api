
#![allow(dead_code, unused_imports)]

use std::sync::Arc;

use crate::hexchat::*;
use crate::hexchat_entry_points::HEXCHAT;
use crate::thread_facilities::*;
use crate::threadsafe_context::*;
use crate::threadsafe_list_iterator::*;

#[derive(Clone, Copy)]
pub struct ThreadSafeHexchat {
    hc: &'static Hexchat,
}

unsafe impl Send for ThreadSafeHexchat {}
unsafe impl Sync for ThreadSafeHexchat {}

impl ThreadSafeHexchat {
    pub (crate) 
    fn new(hc: &'static Hexchat) -> Self {
        ThreadSafeHexchat { hc }
    }
    
    pub fn print(&self, text: &str) {
        let text = std::sync::Arc::new(text.to_string());
        let result = main_thread(move |hc| hc.print(&text));
        result.get();
    }
    
    pub fn command(&self, command: &str) {
        let command = std::sync::Arc::new(command.to_string());
        let result = main_thread(move |hc| hc.command(&command));
        result.get();
    }

    pub fn find_context(&self, 
                        network : &str, 
                        channel : &str
                       ) -> Option<ThreadSafeContext>
    {
        let data = Arc::new((network.to_string(),
                             channel.to_string()));
        main_thread(move |hc| {
            if let Some(ctx) = hc.find_context(&data.0, &data.1) {
                Some(ThreadSafeContext::new(ctx))
            } else {
                None
            }
        }).get()
    }
    
    pub fn get_context(&self) -> Option<ThreadSafeContext> {
        main_thread(|hc| {
            if let Some(ctx) = hc.get_context() {
                Some(ThreadSafeContext::new(ctx))
            } else {
                None
            }
        }).get()
    }
        
    pub fn get_info(&self, id: &str) -> Option<String> {
        let id = Arc::new(id.to_string());
        main_thread(move |hc| {
            hc.get_info(&id)
        }).get()
    }
    pub fn list_get(&self, list: &str) -> Option<ThreadSafeListIterator> {
        let list = Arc::new(list.to_string());
        main_thread(move |hc| {
            if let Some(list_iter) = hc.list_get(&list) {
                Some(ThreadSafeListIterator::new(list_iter))
            } else {
                None
            }
        }).get()
    }
}

