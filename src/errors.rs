use std::error;
use std::fmt;

/// Errors generated directly from the main Object, `Hexchat`.
#[derive(Debug)]
pub enum HexchatError {
    CommandFailed(String),
}
use HexchatError::*;

impl error::Error for HexchatError {}

impl fmt::Display for HexchatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandFailed(message) => {
                write!(f, "A method of `Hexchat` failed with this message: {}",
                           message)
            },
        }
    }
}
/*
impl Error for HexchatError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.side)
    }
}
*/