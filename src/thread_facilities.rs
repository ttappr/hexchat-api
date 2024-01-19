#![cfg(feature = "threadsafe")]

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
use std::error::Error;
use std::fmt::{Display, self, Formatter};
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
type TaskQueue = LinkedList<Box<dyn Task>>;

/// The task queue that other threads use to schedule tasks to run on the
/// main thread. It is guarded by a `Mutex`.
///
static mut TASK_QUEUE: Option<Arc<Mutex<Option<TaskQueue>>>> = None;

/// The main thread's ID is captured and used by `main_thread()` to determine
/// whether it is being called from the main thread or not. If not, the
/// callback can be invoked right away. Otherwise, it gets scheduled.
/// 
pub(crate) static mut MAIN_THREAD_ID: Option<thread::ThreadId> = None;

/// Base trait for items placed on the task queue.
/// 
trait Task : Send {
    fn execute(&mut self, hexchat: &Hexchat);
    fn set_error(&mut self, error: &str);
}

/// A task that executes a closure on the main thread.
/// 
struct ConcreteTask<F, R> 
where 
    F: FnMut(&Hexchat) -> R,
    R: Clone + Send,
{
    callback : F,
    result   : AsyncResult<R>,
}

impl<F, R> ConcreteTask<F, R> 
where
    F: FnMut(&Hexchat) -> R,
    R: Clone + Send,
{
    fn new(callback: F, result: AsyncResult<R>) -> Self {
        ConcreteTask {
            callback,
            result,
        }
    }
}

impl<F, R> Task for ConcreteTask<F, R> 
where
    F: FnMut(&Hexchat) -> R,
    R: Clone + Send,
{
    /// Executes the closure and sets the result.
    /// 
    fn execute(&mut self, hexchat: &Hexchat) {
        self.result.set((self.callback)(hexchat));
    }
    /// When the task queue is being shut down, this will be called to set the
    /// result to an error.
    /// 
    fn set_error(&mut self, error: &str) {
        self.result.set_error(error);
    }
}

unsafe impl<F, R> Send for ConcreteTask<F, R> 
where 
    F: FnMut(&Hexchat) -> R,
    R: Clone + Send,
{}

/// An error type that can be used to indicate that a task failed. Currently,
/// this is only used when the task queue is being shut down. This happens
/// when Hexchat is closing or the addon is being unloaded.
/// 
#[derive(Debug, Clone)]
pub struct TaskError(String);

impl Error for TaskError {}

impl Display for TaskError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "TaskError: {}", self.0)
    }
}

/// A result object that allows callbacks operating on a thread to send their
/// return value to a receiver calling `get()` from another thread. Whether
/// return data needs to be transferred or not, this object can be used to wait
/// on the completion of a callback, thus providing synchronization between
/// threads.
/// 
#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct AsyncResult<T: Clone + Send> {
    data: Arc<(Mutex<(Option<Result<T, TaskError>>, bool)>, Condvar)>,
}

unsafe impl<T: Clone + Send> Send for AsyncResult<T> {}
unsafe impl<T: Clone + Send> Sync for AsyncResult<T> {}

impl<T: Clone + Send> AsyncResult<T> {
    /// Constructor. Initializes the return data to None.
    /// 
    pub (crate)
    fn new() -> Self {
        AsyncResult {
            data: Arc::new((Mutex::new((None, false)), Condvar::new()))
        }
    }
    /// Indicates whether the callback executing on another thread is done or
    /// not. This can be used to poll for the result.
    /// 
    #[allow(dead_code)]
    pub fn is_done(&self) -> bool {
        let (mtx, _) = &*self.data;
        mtx.lock().unwrap().1
    }
    /// Blocking call to retrieve the return data from a callback on another
    /// thread.
    /// 
    pub fn get(&self) -> Result<T, TaskError> {
        let (mtx, cvar) = &*self.data;
        let mut guard   = mtx.lock().unwrap();
        while !guard.1 {
            guard = cvar.wait(guard).unwrap();
        }
        guard.0.take().unwrap()
    }
    /// Sets the return data for the async result. This will unblock the
    /// receiver waiting on the result from `get()`.
    /// 
    pub (crate)
    fn set(&self, result: T) {
        let (mtx, cvar) = &*self.data;
        let mut guard   = mtx.lock().unwrap();
               *guard   = (Some(Ok(result)), true);
        cvar.notify_one();
    }
    fn set_error(&self, error: &str) {
        let (mtx, cvar) = &*self.data;
        let mut guard   = mtx.lock().unwrap();
               *guard   = (Some(Err(TaskError(error.into()))), true);
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
        let arc = unsafe { TASK_QUEUE.as_ref().unwrap() };
        if let Some(queue) = arc.lock().unwrap().as_mut() {
            let task = Box::new(ConcreteTask::new(callback, cln));
            queue.push_back(task);
        } 
        else {
            res.set_error("Task queue has been shut down.");
        }
        //else {
        //    cln.set(callback(hex));
        //}
        // TODO - This approach needs some thought and testing. The commented 
        //        out code above had didn't prevent a hang in Hexchat when the
        //        addon was unloaded. The new code also doesn't seem to help
        //        either. Needs further work.
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
            TASK_QUEUE = Some(Arc::new(Mutex::new(Some(LinkedList::new())))); 
        }
        let hex = unsafe { &*PHEXCHAT };
        
        hex.hook_timer(
            TASK_REST_MSECS,
            move |_hc, _ud| {
                let arc = unsafe { TASK_QUEUE.as_ref().unwrap() };
                if arc.lock().unwrap().is_some() {
                    let mut count = 1;
                    
                    while let Some(mut task) 
                        = arc.lock().unwrap().as_mut()
                             .and_then(|q| q.pop_front()) {
                        task.execute(hex);
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

// TODO - Do something about threads blocked on a .get() on any outstanding
//        AsyncResult's. The current branch is a work in progress trying to
//        figure out how to do this.

/// Called when the an addon is being unloaded. This eliminates the task queue.
/// Any holders of `AsyncResult` objects that are blocked on `.get()` may be
/// waiting forever. This can be called from addons if the thread-safe
/// features aren't going to be utilized. No need to have a timer callback
/// being invoked endlessly doing nothing.
///
pub (crate)
fn main_thread_deinit() {
    if let Some(queue) = unsafe { &TASK_QUEUE } {
        if let Some(mut queue ) = queue.lock().unwrap().take() {
            while let Some(mut task) = queue.pop_front() {
                task.set_error("Task queue is being shut down.");
            }
        }
    }
}

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
/// # Safety
/// While this will disable the handling of the main thread task queue, it
/// doesn't prevent the plugin author from spawning threads and attempting to
/// use the features of the threadsafe objects this crate provides. If the 
/// plugin author intends to use ThreadSafeContext, ThreadSafeListIterator, or
/// invoke `main_thread()` directly, then this function should not be called.
///
#[deprecated(
    since = "0.2.6",
    note = "This function is no longer necessary. Threadsafe features can be\
            turned off by specifying `features = []` in the Cargo.toml file \
            for the `hexchat-api` dependency.")]
pub unsafe fn turn_off_threadsafe_features() {
    main_thread_deinit();
}
