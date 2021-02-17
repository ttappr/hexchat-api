#![allow(dead_code)]

use std::any::Any;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;
use std::sync::Mutex;

/// Represents the user data that is provided to callbacks when they're invoked.
/// A `UserData` object is registered with the callback using one of the
/// hook commands. The user data can be virtually any type that implements the
/// `Any` trait capable of being downcast to its original type. There are
/// four variants for the user data. Which one to use depends on how the
/// callback user data is shared. If the data is unique to one callback, then
/// `BoxedData` should be enough. For single threaded sharing among more than
/// one callback, `SharedData` will do the trick. If, for any odd sort of reason
/// threading somehow becomes relevant to the user data object, `SyncData` would
/// be the variant to use.
///
/// The class has 4 creation functions - one for each variant. And a convenience
/// function that takes a closure that accepts the `UserData`'s wrapped value
/// as a parameter. A good way to avoid all the dereferencing it takes to
/// access the interior objects nested inside the sharing and mutability host
/// objects.
/// # Variants
/// * `BoxedData`   - Can hold user data that only one callback uses.
/// * `SharedData`  - Can allow more than one callback or other code to hold
///                   a cloned copy that references the same user data.
/// * `SyncData`    - Like `SharedData`, but uses the sync objects internally
///                   to allow the user data to be shared among threads.
/// * `NoData`      - Represents the absence of data.
///
#[derive(Debug)]
pub enum UserData {
    BoxedData  ( Box<dyn Any>         ),
    SharedData ( Rc<RefCell<dyn Any>> ),
    SyncData   ( Arc<Mutex<dyn Any>>  ),
    NoData,
}

use UserData::*;

impl UserData {
    /// Creates a `BoxedData` variant. The type to use for user data that
    /// isn't shared between Hexchat callbacks.
    /// # Arguments
    /// * `user_data` - The user data to box.
    /// # Returns
    /// * `BoxedData(user_data)`.
    ///
    pub fn boxed<D:'static>(user_data: D) -> Self {
        BoxedData(Box::new(user_data))
    }

    /// Creates a `SharedData` variant instance. The type to use if the user
    /// data needs to have shared access.
    /// # Arguments
    /// * `user_data` - The user data to wrap internally with `Rc<RefCell<_>>`.
    /// # Returns
    /// `SharedData(user_data)`.
    ///
    pub fn shared<D:'static>(user_data: D) -> Self {
        SharedData(Rc::new(RefCell::new(user_data)))
    }

    /// Creates a `SyncData` variant. The type to use if the user data needs
    /// to be accessible from other threads.
    /// # Arguments
    /// `user_data` - The user data to wrap internally with `Arc<Mutex<_>>`.
    /// # Returns
    /// * `SyncData(user_data)`.
    ///
    pub fn sync<D:'static>(user_data: D) -> Self {
        SyncData(Arc::new(Mutex::new(user_data)))
    }

    /// Applies the given function to the wrapped object inside a `UserData`
    /// object. The type of the wrapped data has to be compatible with the
    /// type of the function's single parameter, or the downcast won't work
    /// and `apply()` will return `None` without invoking the function.
    /// # Arguments
    /// * `f` - A function, or closure, to invoke with the user data, free of
    ///         any wrappers. The format of the function needs to be
    ///         `Fn(&T) -> R`, where `D` is the type of the user data; and `R`
    ///         is the return type that gets wrapped in an `Option` and returned
    ///         by `apply()`.
    /// # Returns
    /// * Returns the return value of function `f` wrapped in an `Option`.
    ///   Or returns `None` if the downcast failed. This can happen if `D`
    ///   is incompatible with the user data's actual type.
    ///
    pub fn apply<D:'static, F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&D) -> R
    {
        match self {
            BoxedData(d) => {
                Some(f(d.downcast_ref::<D>()?))
            },
            SharedData(d) => {
                Some(f(d.borrow().downcast_ref::<D>()?))
            },
            SyncData(d) => {
                Some(f((*d.lock().unwrap()).downcast_ref::<D>()?))
            },
            NoData => { None },
        }
    }

    pub fn take(self) -> Self {
        unimplemented!("FIX THIS!")
        //self
    }
}

impl Clone for UserData {
    fn clone(&self) -> Self {
        match self {
            SharedData(d) => { SharedData(d.clone()) },
            SyncData(d)   => { SyncData(d.clone()) },
            NoData        => { NoData },
            BoxedData(d)  => { 
                panic!("If user data needs to be shared, The `SharedData` or \
                       `SyncData` variants should be used.")
            },
        }
    }
}

impl Default for UserData {
    fn default() -> Self { NoData }
}

