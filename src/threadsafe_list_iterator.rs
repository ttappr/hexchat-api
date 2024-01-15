
//! This module provides a thread-safe wrapper class for the Hexchat 
//! `ListIterator`. The methods it provides can be invoked from threads other
//! than the Hexchat main thread safely.

use std::sync::Arc;
use std::fmt;
use std::sync::RwLock;

use send_wrapper::SendWrapper;

use crate::list_item::*;
use crate::list_iterator::*;
use crate::thread_facilities::*;
use crate::threadsafe_context::*;

/// A thread-safe wrapper class for the Hexchat `ListIterator`. The methods
/// provided, internally execute on the Hexchat main thread without any
/// additional code necessary to make that happen in the client code.
///
/// Objects of this struct can iterate over Hexchat's lists from other threads.
/// Because each operation is delegated to the main thread from the current 
/// thread, they are not going to be as fast as the methods of `ListIterator` 
/// used exclusively in the main thread without switching to other threads.
/// The plus to objects of this struct iterating and printing long lists is they
/// won't halt or lag the Hexchat UI. The list can print item by item, while
/// while Hexchat is able to handle its traffic, printing chat messages, and 
/// other tasks.
///
#[derive(Clone)]
pub struct ThreadSafeListIterator {
    list_iter: Arc<RwLock<Option<SendWrapper<ListIterator>>>>,
}

unsafe impl Send for ThreadSafeListIterator {}
unsafe impl Sync for ThreadSafeListIterator {}

impl ThreadSafeListIterator {
    /// Creates a new wraper object for a `ListIterator`.
    /// # Arguments
    /// * `list_iter` - The list iterator to wrap.
    ///
    pub (crate) 
    fn create(list_iter: ListIterator) -> Self {
        Self { 
            list_iter: Arc::new(RwLock::new(Some(SendWrapper::new(list_iter)))) 
        }
    }
    
    /// This can give unpredictable results if executed from a thread that isn't
    /// Hexchat's main thread. Use a `ThreadSafeContext` to get a list from
    /// non-main threads. With that said, this method executed from the main
    /// thread will produce the list from the current context which can then
    /// be passed to another thread.
    /// # Arguments
    /// * `name` - The name of the list to get.
    /// # Returns
    /// * A thread-safe object representing one of Hexchat's internal lists.
    ///
    pub fn new(name: &str) -> Option<Self> {
        ListIterator::new(name).map(|list| ThreadSafeListIterator {
            list_iter: Arc::new(RwLock::new(Some(SendWrapper::new(list))))
        })
    }
    
    /// Returns a vector of the names of the fields supported by the list
    /// the list iterator represents.
    ///
    pub fn get_field_names(&self) -> Vec<String> {
        let me = self.clone();
        main_thread(move |_| {
            me.list_iter.read().unwrap().as_ref()
                        .expect("ListIterator dropped from threadsafe context.")
                        .get_field_names().to_vec()
        }).get()
    }
    
    /// Constructs a vector of list items on the main thread all at once. The
    /// iterator will be spent after the operation.
    ///
    pub fn to_vec(&self) -> Vec<ListItem> {
        let me = self.clone();
        main_thread(move |_| {
            me.list_iter.read().unwrap().as_ref()
                        .expect("ListIterator dropped from threadsafe context.")
                        .to_vec()
        }).get()
    }
    
    /// Creates a `ListItem` from the field data at the current position in the
    /// list.
    ///
    pub fn get_item(&self) -> ListItem {
        let me = self.clone();
        main_thread(move |_| {
            me.list_iter.read().unwrap().as_ref()
                        .expect("ListIterator dropped from threadsafe context.")
                        .get_item()
        }).get()
    }
    
    /// Returns the value for the field of the requested name.
    ///
    /// # Arguments
    /// * `name` - The name of the field to retrieve the value for.
    ///
    /// # Returns
    /// * A `Result` where `Ok` holds the field data, and `Err` indicates the
    ///   field doesn't exist or some other problem. See `ListError` for the
    ///   error types. The values are returned as `FieldValue` tuples that hold
    ///   the requested data.
    ///
    pub fn get_field(&self, 
                     name: &str
                    ) -> Result<ThreadSafeFieldValue, ListError> 
    {
        use FieldValue as FV;
        use ThreadSafeFieldValue as TSFV;
        
        let name = name.to_string();
        let me = self.clone();
        main_thread(move |_| {
            if let Some(iter) = me.list_iter.read().unwrap().as_ref() {
                match iter.get_field(&name) {
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
            } else {
                Err(ListError::ListIteratorDropped(
                    "ListIterator dropped from threadsafe context."
                    .to_string()))
            }
        }).get()
    }
}

impl Iterator for ThreadSafeListIterator {
    type Item = Self;
    fn next(&mut self) -> Option<Self::Item> {
        let me = self.clone();
        main_thread(move |_| {
            if let Some(iter) = me.list_iter.write().unwrap().as_mut() {
                iter.next().map(|it| ThreadSafeListIterator::create(it.clone()))
            } else {
                None
            }
        }).get()
    }
}

impl Iterator for &ThreadSafeListIterator {
    type Item = Self;
    fn next(&mut self) -> Option<Self::Item> {
        let me = self.clone();
        let has_more = main_thread(move |_| {
            me.list_iter.write().unwrap().as_mut()
                        .map_or(false, |it| it.next().is_some())
        }).get();
        if has_more {
            Some(self)
        } else {
            None
        }
    }
}

impl Drop for ThreadSafeListIterator {
    fn drop(&mut self) {
        if Arc::strong_count(&self.list_iter) == 1
            && self.list_iter.read().unwrap().is_some() {
            let me = self.clone();
            main_thread(move |_| {
                me.list_iter.write().unwrap().take();
            });
        }
    }
}

/// Thread-safe versions of the `FieldValue` variants provided by 
/// `ListIterator`.
/// # Variants
/// * StringVal    - A string has been returned. The enum item holds its value.
/// * IntVal       - Integer value.
/// * PointerVal   - A `u64` value representing the value of a pointer.
/// * ContextVal   - Holds a `ThreadSafeContext` that can be used from other
///                  threads.
/// * TimeVal      - Holds a `i64` value which can be cast to a `time_t` numeric
///                  value.
///
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




