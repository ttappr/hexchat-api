
#![allow(dead_code, unused_imports)]

use std::sync::Arc;
use std::fmt;

use crate::context::*;
use crate::hexchat::Hexchat;
use crate::thread_facilities::*;
use crate::threadsafe_list_iterator::*;

#[derive(Clone, Debug)]
pub struct ThreadSafeContext {
    ctx : Arc<Context>,
}

unsafe impl Send for ThreadSafeContext {}
unsafe impl Sync for ThreadSafeContext {}

impl ThreadSafeContext {

    pub (crate) 
    fn new(ctx: Context) -> Self 
    {
        ThreadSafeContext { ctx: Arc::new(ctx) }
    }

    pub fn print(&self, message: &str) -> Result<(), ContextError> 
    {
        let message = Arc::new(message.to_string());
        let me = self.clone();
        main_thread(move |_| {
            me.ctx.print(&message)
        }).get()
    }
    
    pub fn command(&self, command: &str) -> Result<(), ContextError> 
    {
        let command = Arc::new(command.to_string());
        let me = self.clone();
        main_thread(move |_| {
            me.ctx.command(&command)
        }).get()
    }
    
    pub fn get_info(&self, info: &str) -> Result<Option<String>, ContextError>
    {
        let info = Arc::new(info.to_string());
        let me = self.clone();
        main_thread(move |_| {
            me.ctx.get_info(&info)
        }).get()
    }
    
    pub fn emit_print(&self, event_name: &str, var_args: &[&str])
        -> Result<(), ContextError>
    {
        let var_args: Vec<String> = var_args.iter()
                                            .map(|s| s.to_string())
                                            .collect();
        let data = Arc::new((event_name.to_string(), var_args));
        let me = self.clone();
        main_thread(move |_| {
            let var_args: Vec<&str> = data.1.iter()
                                            .map(|s| s.as_str())
                                            .collect();
            me.ctx.emit_print(&data.0, var_args.as_slice())
        }).get()
    }
    
    pub fn list_get(&self, 
                    name: &str
                   ) -> Result<Option<ThreadSafeListIterator>, ContextError>
    {
        let name = Arc::new(name.to_string());
        let me = self.clone();
        main_thread(move |_| {
            match me.ctx.list_get(&name) {
                Ok(opt) => {
                    if let Some(list) = opt {
                        Ok(Some(ThreadSafeListIterator::new(list)))
                    } else {
                        Ok(None)
                    }
                },
                Err(err) => Err(err),
            }
        }).get()
    }
}

impl fmt::Display for ThreadSafeContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.ctx)
    }
}
