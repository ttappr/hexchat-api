
#![allow(dead_code, unused_imports)]

use std::sync::Arc;
use std::fmt;
use std::iter::IntoIterator;


use crate::hexchat_entry_points::HEXCHAT;
use crate::list_iterator::*;
use crate::thread_facilities::*;
use crate::threadsafe_context::*;

#[derive(Clone)]
pub struct ThreadSafeListIterator {
    list_iter: Arc<ListIterator>,
}

unsafe impl Send for ThreadSafeListIterator {}
unsafe impl Sync for ThreadSafeListIterator {}

impl ThreadSafeListIterator {
    pub (crate) 
    fn new(list_iter: ListIterator) -> Self {
        ThreadSafeListIterator { list_iter: Arc::new(list_iter) }
    }
    
    pub fn get_field_names(&self) -> Vec<String> {
        let me = self.clone();
        main_thread(move |_| {
            me.list_iter.get_field_names().iter().map(|s| s.clone()).collect()
        }).get()
    }

    pub fn get_field(&self, 
                     name: &str
                    ) -> Result<ThreadSafeFieldValue, ListError> 
    {
        use FieldValue as FV;
        use ThreadSafeFieldValue as TSFV;
        
        let name = Arc::new(name.to_string());
        let me = self.clone();
        main_thread(move |_| {
            match me.list_iter.get_field(&name) {
                Ok(field_val) => {
                    match field_val {
                        FV::StringVal(s) => {
                            Ok(TSFV::StringVal(s))
                        },
                        FV::IntVal(i) => {
                            Ok(TSFV::IntVal(i))
                        },
                        FV::PointerVal(pv) => {
                            Ok(TSFV::PointerVal(pv))
                        },
                        FV::ContextVal(ctx) => {
                            Ok(TSFV::ContextVal(
                                ThreadSafeContext::new(ctx)
                            ))
                        },
                        FV::TimeVal(time) => {
                            Ok(TSFV::TimeVal(time))
                        }
                    }
                },
                Err(err) => {
                    Err(err)
                },
            }
        }).get()
    }
}

impl Iterator for ThreadSafeListIterator {
    type Item = Self;
    fn next(&mut self) -> Option<Self::Item> {
        let mut me = self.clone();
        main_thread(move |_| {
            match Arc::get_mut(&mut me.list_iter).unwrap().next() {
                Some(iter) => {
                    Some(ThreadSafeListIterator::new(iter))
                },
                None => None,
            }
        }).get()
    }
}

impl Iterator for &ThreadSafeListIterator {
    type Item = Self;
    fn next(&mut self) -> Option<Self::Item> {
        let mut me = self.clone();
        if main_thread(move |_| {
            match Arc::get_mut(&mut me.list_iter).unwrap().next() {
                Some(_) => true,
                None => false,
            }
        }).get() {
            Some(self)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum ThreadSafeFieldValue {
    StringVal   (String),
    IntVal      (i32),
    PointerVal  (u64),
    ContextVal  (ThreadSafeContext),
    TimeVal     (i64),
}

unsafe impl Send for ThreadSafeFieldValue {}
unsafe impl Sync for ThreadSafeFieldValue {}

impl fmt::Display for ThreadSafeFieldValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ThreadSafeFieldValue::*;
        match self {
            StringVal(s)   => { write!(f, "{}",   s) },
            IntVal(i)      => { write!(f, "{:?}", i) },
            PointerVal(p)  => { write!(f, "{:?}", p) },
            TimeVal(t)     => { write!(f, "{:?}", t) },
            ContextVal(c)  => { write!(f, "ContextVal({})", c) },
        }
    }
}




