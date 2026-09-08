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

    /// The UserData cannot be cast to the specified type.
    UserDataCastError(String),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn display_strips_quotes_and_names_variant() {
        let err = HexchatError::CommandFailed("oops".into());
        let s = err.to_string();
        assert!(!s.contains('"'));
        assert!(s.contains("CommandFailed"));
        assert!(s.contains("oops"));
    }

    #[test]
    fn display_covers_all_variants() {
        let cases = [
            HexchatError::CommandFailed("a".into()),
            HexchatError::InfoNotFound("a".into()),
            HexchatError::ThreadSafeOperationFailed("a".into()),
            HexchatError::UnknownType("a".into()),
            HexchatError::ContextAcquisitionFailed("a".into()),
            HexchatError::ContextOperationFailed("a".into()),
            HexchatError::ContextDropped("a".into()),
            HexchatError::ListNotFound("a".into()),
            HexchatError::ListFieldNotFound("a".into()),
            HexchatError::ListIteratorNotStarted("a".into()),
            HexchatError::ListIteratorDropped("a".into()),
            HexchatError::UserDataCastError("a".into()),
        ];
        for err in cases {
            let s = err.to_string();
            assert!(!s.contains('"'), "quotes should be stripped: {s}");
            assert!(s.contains('a'));
        }
    }

    #[test]
    fn implements_std_error() {
        let err: &dyn Error = &HexchatError::ListNotFound("x".into());
        assert!(err.to_string().contains("ListNotFound"));
    }

    #[test]
    fn is_clone_and_debug() {
        let err = HexchatError::InfoNotFound("chan".into());
        let cloned = err.clone();
        assert_eq!(format!("{err:?}"), format!("{cloned:?}"));
    }
}
