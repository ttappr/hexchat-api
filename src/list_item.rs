
//! A list item for the `ListIterator` and `ThreadSafeListIterator` items that
//! can populate a vector generated using `ThreadSafeListIterator.to_vec()`, 
//! or using the same function of `ListIterator`. `ListItem`s can also be
//! obtained using the `.get_item()` method of the list classes.

use std::collections::BTreeMap;
use std::ops::Index;

use crate::list_iterator::ListIterator;
use crate::list_iterator::FieldValue;

/// An eagerly constructed list item for vectors created from a `ListIterator`.
/// For `ThreadSafeListIterator` it can sometimes be quicker to eagerly convert
/// it to a `Vec<ListItem>` using `ThreadSafeListIterator.to_vec()` and then
/// iterate over the resulting vector. The conversion happens on the main thread
/// and is done all at once.
///
#[derive(Clone)]
pub struct ListItem {
    fields : BTreeMap<String, FieldValue>,
}

unsafe impl Send for ListItem {}
unsafe impl Sync for ListItem {}

impl ListItem {
    /// Construct a new list item.
    ///
    fn new() -> ListItem {
        ListItem { fields: BTreeMap::new() }
    }
    /// Add a field to the list item.
    ///
    fn add(&mut self, name: &str, field: FieldValue) {
        self.fields.insert(name.to_string(), field);
    }
    /// Returns `Some(&FieldValue)` if the field exists in the item, or `None`
    /// instead.
    ///
    pub fn get(&self, name: &str) -> Option<&FieldValue> {
        self.fields.get(name)
    }
}

impl Index<&str> for ListItem {
    type Output = FieldValue;
    /// The `ListItem` class supports indexing operations using the name of
    /// the field. This will panic if the field doesn't exist. Alternatively,
    /// `ListItem.get()` can be used, which returns an option.
    ///
    fn index(&self, i: &str) -> &Self::Output {
        self.fields.get(i).expect("Field doesn't exist.")
    }
}

impl From<ListIterator> for ListItem {
    /// Consructs a list item from the given `ListIterator` instance and 
    /// consumes it. The item is constructed from the fields retrieved from
    /// the iterator at its current position.
    ///
    fn from(list: ListIterator) -> Self {
        let mut item = ListItem::new();
        for name in list.get_field_names() {
            item.add(name, list.get_field(name).unwrap());
        }
        item
    }
}

impl From<&ListIterator> for ListItem {
    /// Constructs a list item from the iterator reference at its current 
    /// position.
    ///
    fn from(list: &ListIterator) -> Self {
        let mut item = ListItem::new();
        for name in list.get_field_names() {
            item.add(name, list.get_field(name).unwrap());
        }
        item
    }
}

