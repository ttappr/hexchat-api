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
    BoxedData  (       Box     < dyn Any >   ),
    SharedData ( Rc  < RefCell < dyn Any > > ),
    SyncData   ( Arc < Mutex   < dyn Any > > ),
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
    /// and `apply()` will panic.
    /// # Arguments
    /// * `f` - A function, or closure, to invoke with the user data, free of
    ///         any wrappers. The format of the function needs to be
    ///         `Fn(&T) -> R`, where `D` is the type of the user data; and `R`
    ///         is the return type that gets wrapped in an `Option` and returned
    ///         by `apply()`.
    /// # Returns
    /// * Returns the return value of function `f` if the downcast is 
    ///   successful.
    ///
    pub fn apply<D:'static, F, R>(&self, f: F) -> R
    where
        F: FnOnce(&D) -> R
    {
        const ERRMSG: &str = "Unable to downcast to requested type.";
        match self {
            BoxedData(d) => {
                f(d.downcast_ref::<D>().expect(ERRMSG))
            },
            SharedData(d) => {
                f(d.borrow().downcast_ref::<D>().expect(ERRMSG))
            },
            SyncData(d) => {
                f((*d.lock().unwrap()).downcast_ref::<D>().expect(ERRMSG))
            },
            NoData => { panic!("Can't downcast `NoData`.") },
        }
    }
    
    /// Same as the `apply()` function except allows mutable access to the
    /// user data contents.
    ///
    pub fn apply_mut<D:'static, F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut D) -> R
    {
        const ERRMSG: &str = "Unable to downcast to requested type.";
        match self {
            BoxedData(d) => {
                f(d.downcast_mut::<D>().expect(ERRMSG))
            },
            SharedData(d) => {
                f(d.borrow_mut().downcast_mut::<D>().expect(ERRMSG))
            },
            SyncData(d) => {
                f((*d.lock().unwrap()).downcast_mut::<D>().expect(ERRMSG))
            },
            NoData => { panic!("Can't downcast `NoData`.") },
        }
    }
}

impl Deref for UserData {
    type Target = dyn Any;
    
    /// `UserData` objects can be dereferenced to get to the `dyn Any` content
    /// within. `downcast_ref::<T>()` can then be used to cast the hosted object
    /// to the required type.
    ///
    fn deref(&self) -> &Self::Target {
        match self {
            BoxedData(ref d) => d,
            SharedData(ref d) => d,
            SyncData(ref d) => d,
            NoData => panic!("Can't deref `NoData`."),
        }
    }
}

impl DerefMut for UserData {
    /// `UserData` objects can be dereferenced to get to the `dyn Any` content
    /// within. `downcast_mut::<T>()` can then be used to cast the hosted object
    /// to the required type.
    ///
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            BoxedData(ref mut d) => d,
            SharedData(ref mut d) => d,
            SyncData(ref mut d) => d,
            NoData => panic!("Can't deref `NoData`."),
        }
    }
}

impl Clone for UserData {
    /// The clone operation for `UserData` allows each variant to be cloned,
    /// except for `BoxedData`. The reason `BoxedData` is prohibited is to 
    /// deter sharing a box between callbacks, as that's not what
    /// a box is meant to be used for. One of the shared variants is more
    /// appropriate to share access to user data.
    ///
    fn clone(&self) -> Self {
        match self {
            SharedData(d) => { SharedData(d.clone()) },
            SyncData(d)   => { SyncData(d.clone()) },
            NoData        => { NoData },
            BoxedData(_)  => { 
                panic!("Can't clone `BoxedData`. If user data needs to be \
                        shared, The `SharedData` or `SyncData` variants of \
                        `UserData` should be used.")
            },
        }
    }
}

impl Default for UserData {
    /// Implemented to support the `take()` operation in `CallbackData` so the
    /// user data can be retrieved with ownership when a callback is 
    /// deregistered. That take operation replaces the user data the callback
    /// owns with the default value (`NoData`).
    ///
    fn default() -> Self { NoData }
}


