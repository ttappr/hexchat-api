
//! This module provides facilities for accessing Hexchat from routines  
//! running on threads other than Hexchat's main thread. 
//! 
//! Hexchat's plugin API isn't inherently thread-safe, however plugins
//! can spawn separate threads and invoke Hexchat's API by placing routines
//! to execute on Hexchat's main thread. 
//! 
//! `main_thread()` makes it easy to declare a function, or closure, that 
//! contains Hexchat API calls. Once executed, it uses the timer feature
//! of Hexchat to delegate. The function or closure can return any sendable
//! cloneable value, and `main_thread()` will pass that back to the calling
//! thread via an `AsyncResult` object. This can either be ignored, and 
//! the thread can continue doing other work, or `AsyncResult.get()` can be
//! invoked on the result object; this call will block until the main thread
//! has finished executing the callback.

use std::sync::{Arc, Condvar, Mutex};

use crate::hexchat::Hexchat;
use crate::hexchat_entry_points::HEXCHAT;
use crate::user_data::*;

use UserData::*;

/// A result object that allows callbacks operating on a thread to send their
/// return value to a receiver calling `get()` from another thread. Whether
/// return data needs to be transferred or not, this object can be used to wait
/// on the completion of a callback, thus providing synchronization between
/// threads.
/// 
#[derive(Clone)]
pub struct AsyncResult<T: Clone + Send> {
    #[allow(clippy::type_complexity)]
    data: Arc< (Mutex<(Option<T>, bool)>, Condvar) >,
    
    // ^^ ((callback-result, is-done), synchronization-object)
    // This is the simplified format of the `data` field above.
}

unsafe impl<T: Clone + Send> Send for AsyncResult<T> {}
unsafe impl<T: Clone + Send> Sync for AsyncResult<T> {}

impl<T: Clone + Send> AsyncResult<T> {
    /// Constructor. Initializes the return data to None.
    pub (crate)
    fn new() -> Self {
        AsyncResult {
            data: Arc::new((Mutex::new((None, false)), 
                            Condvar::new()))
        }
    }
    /// Indicates whether the callback executing on another thread is done or
    /// not. This can be used to poll for the result.
    #[allow(dead_code)]
    pub fn is_done(&self) -> bool {
        let (mtx, _) = &*self.data;
        mtx.lock().unwrap().1
    }
    /// Blocking call to retrieve the return data from a callback on another
    /// thread.
    pub fn get(&self) -> T {
        let (mtx, cvar) = &*self.data;
        let mut guard   = mtx.lock().unwrap();
        while !(*guard).1 {
            guard = cvar.wait(guard).unwrap();
        }
        (*guard).0.as_ref().unwrap().clone()
    }
    /// Sets the return data for the async result. This will unblock the
    /// receiver waiting on the result from `get()`.
    pub (crate)
    fn set(&self, result: T) {
        let (mtx, cvar) = &*self.data;
        let mut guard   = mtx.lock().unwrap();
               *guard   = (Some(result), true);
        cvar.notify_one();
    }
}

/// Executes a closure from the Hexchat main thread. This function returns
/// immediately with an AsyncResult object that can be used to retrieve the
/// result of the operation that will run on the main thread.
/// 
/// # Arguments
/// * `callback` - The callback to execute on the main thread.
/// 
pub fn _main_thread<F, R>(mut callback: F) -> AsyncResult<R>
where
    F: FnMut(&Hexchat) -> R,
    F: 'static + Send,
    R: 'static + Clone + Send,
{
    let res = AsyncResult::new();
    let cln = res.clone();
    let hex = unsafe { &*HEXCHAT };
    hex.hook_timer(0,
                   move |hc, _ud| {
                        cln.set(callback(hc));
                        0 // Returning 0 disposes of the callback.
                    }, 
                    NoData);
    res
}

// TODO - At some point, figure out if both these functions are needed, or if
//        only one of them serves for all use cases needed.

// TODO - Test the thread safety of multiple threads hammering away on
//        Hexchat's timer feature. If this turns out not to be safe, then
//        create a lock to protect access to it. I believe I've tested this
//        before with my Python lib and it turned out to be safe. I want to test
//        it again with this new Rust lib.

/// Serves the same purpose as `main_thread()` but takes a `FnOnce()` callback
/// instead of `FnMut()`. With the other command, the callback will hold its
/// state between uses. In this case, the callback will be newly initialized
/// each time this command is invoked.
pub fn main_thread_once<F, R>(callback: F) -> AsyncResult<R>
where
    F: FnOnce(&Hexchat) -> R,
    F: 'static + Send,
    R: 'static + Clone + Send,
{
    let res = AsyncResult::new();
    let cln = res.clone();
    let hex = unsafe { &*HEXCHAT };
    hex.hook_timer_once(0,
                        Box::new(
                            move |hc, _ud| {
                                cln.set(callback(hc));
                                0 // Returning 0 disposes of the callback.
                        }),
                        NoData);
    res
}

// TODO - This is an idea to work out. Instead of trusting Hexchat's timer
//        queue to be thread-safe, make a tasks that reads from a queue of
//        callbacks and AsyncResult objects and executes the callbacks.
//        It can set itself to wake up at varying timeouts according to how
//        fast items are added to its queue. Its queue can be guarded by
//        Mutex.
//

use std::collections::LinkedList;

type Queue = LinkedList<Box<dyn FnMut()>>;
const TASK_SPURT_SIZE: i32 = 5;
const TASK_REST_MSECS: i64 = 2;

static mut TASK_QUEUE: Option<Arc<Mutex<Queue>>> = None;

pub (crate)
fn main_thread_init() {
    if unsafe { TASK_QUEUE.is_none() } {
        unsafe { 
            TASK_QUEUE = Some(Arc::new(Mutex::new(LinkedList::new()))); 
        }
        let hex = unsafe { &*HEXCHAT };
        
        hex.hook_timer(
            TASK_REST_MSECS,
            move |_hc, _ud| {
                if let Some(task_queue) = unsafe { &TASK_QUEUE } {
                    let mut count = 0;
                    
                    while let Some(mut callback) = task_queue.lock()
                                                             .unwrap()
                                                             .pop_front() 
                    {
                        callback();
                        count += 1;
                        if count > TASK_SPURT_SIZE { 
                            break  
                        }
                    }
                    1
                } else {
                    0
                }
            },
            NoData);
    }
}

pub (crate)
fn main_thread_deinit() {
    unsafe { TASK_QUEUE = None }
}

pub fn main_thread<F, R>(mut callback: F) -> AsyncResult<R>
where 
    F: FnMut(&Hexchat) -> R,
    F: 'static + Send,
    R: 'static + Clone + Send,
{
    let res = AsyncResult::new();
    let cln = res.clone();
    let hex = unsafe { &*HEXCHAT };
    if let Some(task_queue) = unsafe { &TASK_QUEUE } {
        let cbk = Box::new(
            move || {
                cln.set(callback(hex));
            }
        );
        task_queue.lock().unwrap().push_back(cbk);
    } else {
        cln.set(callback(hex));
    }
    res
}


















