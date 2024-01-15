
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

use std::collections::LinkedList;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use crate::hexchat::Hexchat;
use crate::hexchat_entry_points::PHEXCHAT;
use crate::user_data::*;

use UserData::*;

const TASK_SPURT_SIZE: i32 = 5;
const TASK_REST_MSECS: i64 = 2;

// The type of the queue that closures will be added to and pulled from to run 
// on the main thread of Hexchat.
type TaskQueue = LinkedList<Box<dyn FnMut() + Sync + Send>>;

/// The task queue that other threads use to schedule tasks to run on the
/// main thread. It is guarded by a `Mutex`.
///
static mut TASK_QUEUE: Option<Arc<Mutex<TaskQueue>>> = None;

/// The main thread's ID is captured and used by `main_thread()` to determine
/// whether it is being called from the main thread or not. If not, the
/// callback can be invoked right away. Otherwise, it gets scheduled.
/// 
static mut MAIN_THREAD_ID: Option<thread::ThreadId> = None;

/// Stops and removes the main thread task queue handler. Otherwise it will
/// keep checking the queue while doing nothing useful - which isn't 
/// necessarily bad. Performance is unaffected either way.
///
/// Support for `main_thread()` is on by default. After this function is 
/// invoked, `main_thread()` should not be used and threads in general risk
/// crashing the software if they try to access Hexchat directly without
/// the `main_thread()`. `ThreadSafeContext` and `ThreadSafeListIterator` 
/// should also not be used after this function is called, since they rely on 
/// `main_thread()` internally.
///
pub fn turn_off_threadsafe_features() {
    main_thread_deinit();
}

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
        while !guard.1 {
            guard = cvar.wait(guard).unwrap();
        }
        guard.0.as_ref().unwrap().clone()
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
pub fn main_thread<F, R>(mut callback: F) -> AsyncResult<R>
where 
    F: FnMut(&Hexchat) -> R + Sync + Send,
    F: 'static + Send,
    R: 'static + Clone + Send,
{
    if Some(thread::current().id()) == unsafe { MAIN_THREAD_ID } {
        let result = callback(unsafe { &*PHEXCHAT });
        let res = AsyncResult::new();
        res.set(result);
        res
    } else {
        let res = AsyncResult::new();
        let cln = res.clone();
        let hex = unsafe { &*PHEXCHAT };
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
}

/// This initializes the fundamental thread-safe features of this library.
/// A mutex guarded task queue is created, and a timer function is registered
/// that handles the queue at intervals. If a thread requires fast response,
/// the handler will field its requests one after another for up to 
/// `TASK_SPURT_SIZE` times without rest.
///
pub (crate)
fn main_thread_init() {
    unsafe { MAIN_THREAD_ID = Some(thread::current().id()) }
    if unsafe { TASK_QUEUE.is_none() } {
        unsafe { 
            TASK_QUEUE = Some(Arc::new(Mutex::new(LinkedList::new()))); 
        }
        let hex = unsafe { &*PHEXCHAT };
        
        hex.hook_timer(
            TASK_REST_MSECS,
            move |_hc, _ud| {
                if let Some(task_queue) = unsafe { &TASK_QUEUE } {
                    let mut count = 1;
                    
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
                    1 // Keep going.
                } else {
                    0 // Task queue is gone, remove timer callback.
                }
            },
            NoData);
    }
}

// TODO - Make the thread-safe features optional, so we won't have the queue
//        timer handler needlessly running if an addon doesn't use threads.

// TODO - Do something about threads blocked on a .get() on any outstanding
//        AsyncResult's.

/// Called when the an addon is being unloaded. This eliminates the task queue.
/// Any holders of `AsyncResult` objects that are blocked on `.get()` may be
/// waiting forever. This can be called from addons if the thread-safe
/// features aren't going to be utilized. No need to have a timer callback
/// being invoked endlessly doing nothing.
///
pub (crate)
fn main_thread_deinit() {
    unsafe { TASK_QUEUE = None }
}



















