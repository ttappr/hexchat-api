use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Each of the various ways the API can fail is collected in this enumerated
/// type.
/// 
#[derive(Debug, Clone)]
pub enum HexchatError {
    /// The command failed to execute.
    CommandFailed(String),

    /// The requested info wasn't found or doesn't exist.
    InfoNotFound(String),

    /// This can happen when a `ThreadSafeContext` or `ThreadSafeListIterator`
    /// object is used while the plugin is unloading. The main thread task
    /// handler isn't accepting any more tasks, so the operation fails.
    ThreadSafeOperationFailed(String),

    /// The list iterator may return this if the Hexchat API changes. Currently
    /// this won't get thrown.
    UnknownType(String),

    /// The function was unable to acquire the desired context associated with
    /// the given network and channel names.
    ContextAcquisitionFailed(String),

    /// The context acquisition succeeded, but there is some problem with the
    /// action being performed. For instance the requested list for 
    /// `ctx.get_listiter("foo")` doesn't exist.
    ContextOperationFailed(String),

    /// The context object was dropped.
    ContextDropped(String),

    /// The requested list doesn't exist.
    ListNotFound(String),

    /// The requested field doesn't exist.
    ListFieldNotFound(String),

    /// The list iterator type for Hexchat requires that next() be called at
    /// least once before its fields are accessible.
    ListIteratorNotStarted(String),

    /// The list iterator object was dropped. This might happen if the plugin is
    /// unloading while another thread is still running and using the iterator.
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
