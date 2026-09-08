use std::any::Any;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::RwLock;

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
    BoxedData  ( Box < RefCell < dyn Any > > ),
    SharedData ( Rc  < RefCell < dyn Any > > ),
    SyncData   ( Arc < RwLock  < dyn Any > > ),
    NoData,
}

unsafe impl Send for UserData {}

use UserData::*;

use crate::HexchatError;

impl UserData {
    /// Creates a `BoxedData` variant. The type to use for user data that
    /// isn't shared between Hexchat callbacks.
    /// # Arguments
    /// * `user_data` - The user data to box.
    /// # Returns
    /// * `BoxedData(user_data)`.
    ///
    pub fn boxed<D:'static>(user_data: D) -> Self {
        BoxedData(Box::new(RefCell::new(user_data)))
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
        SyncData(Arc::new(RwLock::new(user_data)))
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
                f(d.borrow().downcast_ref::<D>().expect(ERRMSG))
            },
            SharedData(d) => {
                f(d.borrow().downcast_ref::<D>().expect(ERRMSG))
            },
            SyncData(d) => {
                f((*d.read().unwrap()).downcast_ref::<D>().expect(ERRMSG))
            },
            NoData => { panic!("Can't downcast `NoData`.") },
        }
    }

    /// Same as the `apply()` function except allows mutable access to the
    /// user data contents.
    ///
    pub fn apply_mut<D:'static, F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut D) -> R
    {
        const ERRMSG: &str = "Unable to downcast to requested type.";
        match self {
            BoxedData(d) => {
                f(d.borrow_mut().downcast_mut::<D>().expect(ERRMSG))
            },
            SharedData(d) => {
                f(d.borrow_mut().downcast_mut::<D>().expect(ERRMSG))
            },
            SyncData(d) => {
                f((*d.write().unwrap()).downcast_mut::<D>().expect(ERRMSG))
            },
            NoData => { panic!("Can't downcast `NoData`.") },
        }
    }

    /// Retrieves the user data from the `UserData` object. The type of the
    /// user data must be the same as the type of the `UserData` object, or
    /// the downcast will fail and return an error. The returned data is a
    /// clone of the original value.
    ///
    /// # Generic Arguments
    /// * `D` - The type of the user data to retrieve.
    ///
    /// # Returns
    /// * `Ok(user_data)` - The user data if the downcast is successful.
    /// * `Err(HexchatError::UserDataCastError)` - The error if the downcast
    ///   fails.
    ///
    pub fn get<D: 'static + Clone>(&self) -> Result<D, HexchatError> {
        use HexchatError::UserDataCastError;
        const ERRMSG: &str = "UserData::get() - Unable to downcast to \
                              requested type.";
        match self {
            BoxedData(d) => {
                d.borrow()
                 .downcast_ref::<D>()
                 .cloned()
                 .ok_or_else(|| UserDataCastError(ERRMSG.into()))
            },
            SharedData(d) => {
                d.borrow()
                 .downcast_ref::<D>()
                 .cloned()
                 .ok_or_else(|| UserDataCastError(ERRMSG.into()))
            },
            SyncData(d) => {
                d.read().unwrap()
                 .downcast_ref::<D>()
                 .cloned()
                 .ok_or_else(|| UserDataCastError(ERRMSG.into()))
            }
            NoData => {
                Err(UserDataCastError("`UserData::NoData` can't be cast."
                                      .into()))
            }
        }
    }

    /// Sets the user data in the `UserData` object. The type of the user data
    /// must be the same as the type of the `UserData` object, or the downcast
    /// will fail and return an error.
    ///
    /// # Generic Arguments
    /// * `D` - The type of the user data to set.
    ///
    /// # Arguments
    /// * `value` - The value to set the user data to.
    ///
    /// # Returns
    /// * `Ok(())` - The user data if the downcast is successful.
    /// * `Err(HexchatError::UserDataCastError)` - The error if the downcast
    ///   fails.
    ///
    pub fn set<D: 'static>(&self, value: D) -> Result<(), HexchatError> {
        use HexchatError::UserDataCastError;
        const ERRMSG: &str = "`UserData::set()` - Unable to downcast to \
                              requested type.";
        let mut success = false;
        match self {
            BoxedData(d) => {
                if let Some(d) = d.borrow_mut().downcast_mut::<D>() {
                    *d = value;
                    success = true;
                }
            },
            SharedData(d) => {
                if let Some(d) = d.borrow_mut().downcast_mut::<D>() {
                    *d = value;
                    success = true;
                }
            },
            SyncData(d) => {
                if let Some(d) = d.write().unwrap().downcast_mut::<D>() {
                    *d = value;
                    success = true;
                }
            }
            NoData => {
                return Err(UserDataCastError("`UserData::NoData` can't be cast"
                                             .into()));
            }
        }
        if success {
            Ok(())
        } else {
            Err(UserDataCastError(ERRMSG.into()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxed_apply_get_set_round_trip() {
        let ud = UserData::boxed(42i32);
        assert_eq!(ud.apply(|v: &i32| *v), 42);
        assert_eq!(ud.get::<i32>().unwrap(), 42);

        ud.set(7i32).unwrap();
        assert_eq!(ud.get::<i32>().unwrap(), 7);

        // Wrong-type set fails.
        assert!(ud.set("not an i32").is_err());
        // Wrong-type get fails.
        assert!(ud.get::<String>().is_err());
    }

    #[test]
    fn boxed_apply_mut_mutates_in_place() {
        let ud = UserData::boxed(String::from("hi"));
        ud.apply_mut(|s: &mut String| s.push('!'));
        assert_eq!(ud.get::<String>().unwrap(), "hi!");
    }

    #[test]
    fn shared_clone_shares_state() {
        let ud = UserData::shared(10i32);
        let clone = ud.clone();
        clone.set(99i32).unwrap();
        // Both handles observe the same interior value.
        assert_eq!(ud.get::<i32>().unwrap(), 99);
        assert_eq!(clone.apply(|v: &i32| *v), 99);
    }

    #[test]
    fn shared_apply_mut_works() {
        let ud = UserData::shared(vec![1i32]);
        ud.apply_mut(|v: &mut Vec<i32>| v.push(2));
        assert_eq!(ud.get::<Vec<i32>>().unwrap(), vec![1, 2]);
    }

    #[test]
    fn sync_clone_shares_across_handles() {
        let ud = UserData::sync(1u64);
        let clone = ud.clone();
        assert_eq!(ud.apply(|v: &u64| *v), 1);
        clone.set(2u64).unwrap();
        assert_eq!(ud.get::<u64>().unwrap(), 2);
    }

    #[test]
    fn sync_send_across_threads() {
        let ud = UserData::sync(0i32);
        let other = ud.clone();
        let handle = std::thread::spawn(move || {
            other.set(5i32).unwrap();
            other.get::<i32>().unwrap()
        });
        assert_eq!(handle.join().unwrap(), 5);
        assert_eq!(ud.get::<i32>().unwrap(), 5);
    }

    #[test]
    fn no_data_get_and_set_fail() {
        let ud = UserData::NoData;
        assert!(ud.get::<i32>().is_err());
        assert!(ud.set(1i32).is_err());
    }

    #[test]
    #[should_panic(expected = "Can't downcast `NoData`")]
    fn no_data_apply_panics() {
        let ud = UserData::NoData;
        let _ = ud.apply(|_: &i32| 0);
    }

    #[test]
    #[should_panic(expected = "Can't downcast `NoData`")]
    fn no_data_apply_mut_panics() {
        let ud = UserData::NoData;
        let _ = ud.apply_mut(|_: &mut i32| 0);
    }

    #[test]
    #[should_panic(expected = "Unable to downcast")]
    fn apply_wrong_type_panics() {
        let ud = UserData::boxed(1i32);
        let _ = ud.apply(|_: &String| 0);
    }

    #[test]
    #[should_panic(expected = "Can't clone `BoxedData`")]
    fn boxed_clone_panics() {
        let ud = UserData::boxed(1i32);
        let _ = ud.clone();
    }

    #[test]
    fn clone_no_data_gives_no_data() {
        let ud = UserData::NoData;
        assert!(matches!(ud.clone(), UserData::NoData));
    }

    #[test]
    fn default_is_no_data() {
        assert!(matches!(UserData::default(), UserData::NoData));
    }
}


