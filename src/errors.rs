use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Each of the various ways the API can fail is collected in this enumerated
/// type.
/// 
/// # Variants
/// * `CommandFailed`       - The command failed to execute.
/// * `InfoNotFound`        - The requested info wasn't found or doesn't exist.
/// * `ThreadSafeOperationFailed` 
///                         - This can happen when a `ThreadSafeContext` or 
///                          `ThreadSafeListIterator` object is used while the
///                           plugin is unloading.
/// * `UnknownType`         - The list iterator may return this if the Hexchat
///                           API changes. Currently this won't get thrown.
/// * `ContextAcquisitionFailed`   
///                         - The function was unable to acquire the desired
///                           context associated with its network and channel
///                           names.
/// * `ContextOperationFailed`     
///                         - The context acquisition succeeded, but there is
///                           some problem with the action being performed,
///                           for instance the requested list for
///                           `ctx.get_listiter("foo")` doesn't exist.
/// * `ContextDropped`      - The context object was dropped.
///                           the plugin is unloading.
/// * `ListNotFound`        - The requested list doesn't exist.
/// * `ListFieldNotFound`   - The requested field doesn't exist.
/// * `ListIteratorNotStarted`
///                         - The list iterator type for Hexchat requires that
///                           next() be called at least once before its fields 
///                           are accessible.
/// * `ListIteratorDropped` - The list iterator object was dropped. This might
///                           happen if the plugin is unloading while another
///                           thread is still running and using the iterator.
#[derive(Debug, Clone)]
pub enum HexchatError {
    CommandFailed(String),
    InfoNotFound(String),
    ThreadSafeOperationFailed(String),
    UnknownType(String),

    ContextAcquisitionFailed(String),
    ContextOperationFailed(String),
    ContextDropped(String),

    ListNotFound(String),
    ListFieldNotFound(String),
    ListIteratorNotStarted(String),
    ListIteratorDropped(String),
}

unsafe impl Send for HexchatError {}

impl Error for HexchatError {}

impl Display for HexchatError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut s = format!("{:?}", self);
        s.retain(|c| c != '"');
        write!(f, "{}", s)
    }
}
