
//! This module provides a Rust wrapper for Hexchat's list related API. A list
//! can be accessed by creating a `ListIterator` by passing the name of one of
//! the list types to the constructor. The iterator itself can be used to
//! access list item fields by name using `get_field()`. Fields can't be
//! accessed until `next()` has been invoked to advance the internal pointer.
//! This iterator can be used in loops like any other iterator, or `collect()`
//! can be called to generate a vector or other collections.

use libc::c_void;
use libc::time_t;
use core::panic;
use std::cell::RefCell;
use std::fmt;
#[cfg(feature = "threadsafe")]
use std::thread;
use std::rc::Rc;

#[cfg(feature = "threadsafe")]
use crate::MAIN_THREAD_ID;
use crate::context::*;
use crate::errors::HexchatError;
use crate::hexchat::Hexchat;
use crate::hexchat_entry_points::PHEXCHAT;
use crate::list_item::ListItem;
use crate::utils::*;

// Local types.
use FieldValue::*;
use HexchatError::*;

/// The `ListIterator` wraps the list pointer and related functions of Hexchat.
/// It provides are more Rust OO interface. The iterator returns clones of
/// itself that can be used to access the current list item's fields through
/// `get_field()`. The list iterator object is internally a smart pointer,
/// among other things. You can clone it if you need multiple references to
/// a list.
#[derive(Clone)]
pub struct ListIterator {
    field_names : Rc<Vec<String>>,
    data        : Rc<RefCell<ListIteratorData>>,
}

impl ListIterator {
    /// Creates a new list iterator instance.`
    /// # Arguments
    /// * `list_name` - The name of one of the Hexchat lists ('channels', 'dcc',
    ///                'ignore', 'notify', 'users').
    /// # Returns
    /// * An iterator to the list of the requested name, or `None` if the list
    ///   doesn't exist.
    ///
    pub fn new(list_name: &str) -> Option<Self> {
        #[cfg(feature = "threadsafe")]
        assert!(thread::current().id() == unsafe { MAIN_THREAD_ID.unwrap() },
                "ListIterator::new() must be called from the main thread.");
        let name     = str2cstring(list_name);
        let hc       = unsafe { &*PHEXCHAT };
        let list_ptr = unsafe { (hc.c_list_get)(hc, name.as_ptr()) };
        if !list_ptr.is_null() {
            let mut field_types = vec![];
            let mut field_names = vec![];
            unsafe {
                // Get the list pointer to field names.
                let     c_fields = (hc.c_list_fields)(hc, name.as_ptr());
                let mut c_field  = *c_fields;
                let mut i        = 0;

                // Build a mapping between field names and field types.
                while !c_field.is_null() && *c_field != 0 {
                    // The first char of the name is the type.
                    let c_typ = *c_field;
                    // Advance the char pointer once to get the name w/o type
                    // char.
                    let field = pchar2string(c_field.add(1));

                    field_types.push((field.clone(), c_typ));
                    field_names.push(field);
                    i += 1;
                    c_field = *c_fields.add(i);
                }
                field_names.sort();
            }
            Some( ListIterator {
                    field_names: Rc::new(field_names),
                    data: Rc::new(
                        RefCell::new(
                            ListIteratorData {
                                list_name : list_name.to_string(),
                                hc,
                                field_types,
                                list_ptr,
                                started: false,
                            }))})
        } else {
            None
        }
    }

    /// Eagerly constructs a vector of `ListItem`s. The iterator will be spent
    /// afterward.
    ///
    pub fn to_vec(&self) -> Vec<ListItem> {
        self.map(ListItem::from).collect()
    }

    /// Creates a `ListItem` from the field data at the current position in
    /// the list.
    ///
    pub fn get_item(&self) -> ListItem {
        ListItem::from(self)
    }

    /// Returns a slice containing the field names of the list items.
    ///
    pub fn get_field_names(&self) -> &[String] {
        &self.field_names
    }

    /// Returns the value for the field of the requested name.
    ///
    /// # Arguments
    /// * `name` - The name of the field to retrieve the value for.
    ///
    /// # Returns
    /// * A `Result` where `Ok` holds the field data, and `Err` indicates the
    ///   field doesn't exist or some other problem. See [HexchatError] for the
    ///   error types. The values are returned as `FieldValue` tuples that hold
    ///   the requested data.
    ///
    pub fn get_field(&self, name: &str) -> Result<FieldValue, HexchatError> {
        let cell = &*self.data;
        let data = &*cell.borrow();
        if data.started {
            let field_type_opt = data.get_type(name);
            if let Some(field_type) = field_type_opt {
                self.get_field_pvt(data, name, field_type)
            } else {
                Err(ListFieldNotFound(name.to_owned()))
            }
        } else {
            Err(ListIteratorNotStarted("The iterator must have `.next()` \
                                       invoked before fields can be accessed."
                                       .to_string()))
        }
    }

    /// Traverses a list while invoking the supplied callback to give the
    /// record data. This is an alternative push model approach to accessing
    /// the list data sequentially. The visitor callback has the form:
    ///
    /// ```FnMut(&String, &FieldValue, bool) -> bool```
    ///
    /// The first parameter is the field name, followed by its value,
    /// then a boolean that when `true` indicates the start of a new record.
    /// The callback returns `true` to keep going. If it returns `false`,
    /// the traversal stops.
    ///
    pub fn traverse<F>(&self, mut visitor: F)
    where
        F: FnMut(&String, &FieldValue, bool) -> bool
    {
        let cell = &*self.data;

        'main: for _item in self {
            let data = &*cell.borrow();
            let mut start = true;
            for (field_name, field_type) in &data.field_types {
                let value = self.get_field_pvt(data, field_name, *field_type)
                                .unwrap();
                if !visitor(field_name, &value, start) {
                    break 'main;
                }
                start = false;
            }
        }
    }

    /// Internal method that gets the value of a field given the field name
    /// and type. This should remain private in scope to this file as using
    /// the wrong type when accessing fields can cause instability. This method
    /// is invoked by `traverse()` and `get_field()`.
    ///
    fn get_field_pvt(&self, data: &ListIteratorData, name: &str, field_type: i8)
        -> Result<FieldValue, HexchatError>
    {
        let c_name = str2cstring(name);
        unsafe {
            match field_type {
                // Match against the ascii values for one of 's', 'i',
                //'p', or 't'.
                115 /* 's' (string) */ => {
                    let val = (data.hc.c_list_str)(data.hc,
                                                   data.list_ptr,
                                                   c_name.as_ptr());
                    Ok(StringVal(pchar2string(val)))
                },
                105 /* 'i' (integer) */ => {
                    let val = (data.hc.c_list_int)(data.hc,
                                                   data.list_ptr,
                                                   c_name.as_ptr());
                    Ok(IntVal(val))
                },
                112 /* 'p' (pointer) */ => {
                    let networkcstr = str2cstring("network");
                    let channelcstr = str2cstring("channel");
                    if name.to_lowercase() == "context" {
                        let network = (data.hc.c_list_str)(data.hc,
                                                           data.list_ptr,
                                                           networkcstr
                                                           .as_ptr());
                        let channel = (data.hc.c_list_str)(data.hc,
                                                           data.list_ptr,
                                                           channelcstr
                                                           .as_ptr());
                        if let Some(c) = Context::find(&pchar2string(network),
                                                       &pchar2string(channel))
                        {
                            Ok(ContextVal(c))
                        } else {
                            Err(ContextAcquisitionFailed("Context unavailable."
                                                         .to_string()))
                        }
                    } else {
                        let ptr = (data.hc.c_list_str)(data.hc,
                                                       data.list_ptr,
                                                       c_name.as_ptr());
                        Ok(PointerVal(ptr as u64))
                    }
                },
                116 /* 't' (time) */ => {
                    let val = (data.hc.c_list_time)(data.hc,
                                                    data.list_ptr,
                                                    c_name.as_ptr());
                    Ok(TimeVal(val))
                },
                _ => {
                    // This should never happen.
                    Err(UnknownType(field_type.to_string()))
                },
            }
        }
    }
}

impl Iterator for ListIterator {
    type Item = Self;

    /// The standard method for iterators. The items returned are clones of the
    /// iterator itself. Calling `next` on the iterator advances an internal
    /// pointer used to access Hexchat data.
    ///
    fn next(&mut self) -> Option<Self::Item> {
        let data = &mut *self.data.borrow_mut();
        data.started = true;
        if unsafe { (data.hc.c_list_next)(data.hc, data.list_ptr) != 0 } {
            Some(self.clone())
        } else {
            None
        }
    }
}

impl Iterator for &ListIterator {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        let data = &mut *self.data.borrow_mut();
        data.started = true;
        if unsafe { (data.hc.c_list_next)(data.hc, data.list_ptr) != 0 } {
            Some(self)
        } else {
            None
        }
    }
}

/// Holds the iterator state and maps the field names to their data type.
/// # Fields
/// * `field_types` - A mapping of field names to data type.
/// * `field_names` - The list of field names for the particular list.
/// * `list_ptr`    - A raw pointer to a list internal to Hexchat.
/// * `hc`          - The Hexchat pointer.
/// * `started`     - true if `next()` has aready been called on the Rust iter.
///
#[allow(dead_code)]
struct ListIteratorData {
    list_name   : String,
    field_types : Vec<(String, i8)>,
    hc          : &'static Hexchat,
    list_ptr    : *const c_void,
    started     : bool,
}

impl ListIteratorData {
    /// Returns the type of the given field. The field lists are short, so
    /// a simple comparisons search for the right item may be quicker than
    /// a HashMap's hashings and lookups.
    #[inline]
    fn get_type(&self, field: &str) -> Option<i8> {
        let fields = &self.field_types;
        Some(fields.iter().find(|f| f.0 == field)?.1)
    }
}

impl Drop for ListIteratorData {
    /// Frees the Hexchat internal list pointer.
    fn drop(&mut self) {
        unsafe {
            (self.hc.c_list_free)(self.hc, self.list_ptr);
        }
    }
}

/// # Field Data Types
/// * String    - A string has been returned. The enum item holds its value.
/// * Int       - Integer value.
/// * Pointer   - This will be updated to be Context soon.
/// * Time      - Holds a `time_t` numeric value.
///
#[derive(Debug, Clone)]
pub enum FieldValue {
    StringVal    (String),
    IntVal       (i32),
    PointerVal   (u64),
    ContextVal   (Context),
    TimeVal      (time_t),
}

impl FieldValue {
    /// Convert a StringVal variant to a String. FieldValue also implements
    /// `From<String>`` so you can also use `let s: String = fv.into();` 
    /// to convert.
    /// 
    pub fn str(self) -> String {
        match self {
            StringVal(s) => s,
            _ => panic!("Can't convert {:?} to String.", self),
        }
    }
    /// Convert an IntVal variant to an i32. FieldValue also implements
    /// `From<i32>` so you can also use `let i: i32 = fv.into();`
    /// to convert.
    /// 
    pub fn int(self) -> i32 {
        match self {
            IntVal(i) => i,
            _ => panic!("Can't convert {:?} to i32.", self),
        }
    }
    /// Convert a PointerVal variant to a u64. FieldValue also implements
    /// `From<u64>` so you can also use `let p: u64 = fv.into();`
    /// to convert.
    /// 
    pub fn ptr(self) -> u64 {
        match self {
            PointerVal(p) => p,
            _ => panic!("Can't convert {:?} to u64.", self),
        }
    }
    /// Convert a TimeVal variant to a time_t (i64). FieldValue also implements
    /// `From<time_t>` so you can also use `let t: time_t = fv.into();`
    /// to convert.
    /// 
    pub fn time(self) -> time_t {
        match self {
            TimeVal(t) => t,
            _ => panic!("Can't convert {:?} to time_t.", self),
        }
    }
    /// Convert a ContextVal variant to a Context. FieldValue also implements
    /// `From<Context>` so you can also use `let c: Context = fv.into();`
    /// to convert.
    /// 
    pub fn ctx(self) -> Context {
        match self {
            ContextVal(c) => c,
            _ => panic!("Can't convert {:?} to Context.", self),
        }
    }
}

impl From<FieldValue> for String {
    fn from(v: FieldValue) -> Self {
        v.str()
    }
}

impl From<FieldValue> for i32 {
    fn from(v: FieldValue) -> Self {
        v.int()
    }
}

impl From<FieldValue> for u64 {
    fn from(v: FieldValue) -> Self {
        v.ptr()
    }
}

impl From<FieldValue> for i64 {
    fn from(v: FieldValue) -> Self {
        v.time()
    }
}

impl From<FieldValue> for Context {
    fn from(v: FieldValue) -> Self {
        v.ctx()
    }
}


impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StringVal(s)   => { write!(f, "{}",   s) },
            IntVal(i)      => { write!(f, "{:?}", i) },
            PointerVal(p)  => { write!(f, "{:?}", p) },
            TimeVal(t)     => { write!(f, "{:?}", t) },
            ContextVal(c)  => { write!(f, "ContextVal({})", c) },
        }
    }
}
