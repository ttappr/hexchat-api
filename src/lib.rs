
//! This crate provides a Rust interface to the 
//! [Hexchat Plugin Interface](https://hexchat.readthedocs.io/en/latest/plugins.html)
//! The primary object of the interface is 
//! [Hexchat](https://ttappr.github.io/hexchat_api/hexchat_api/struct.Hexchat.html),
//! which exposes an interface with functions that mirror the C functions 
//! listed on the Hexchat docs page linked above.  

mod hook;
mod callback_data;
mod consts;
mod context;
mod hexchat;
mod hexchat_callbacks;
mod hexchat_entry_points;
mod list_iterator;
mod plugin;
mod thread_facilities;
mod user_data;
mod utils;

pub use hook::*;
//pub use callback_data::*;
pub use consts::*;
pub use context::*;
pub use hexchat::*;
//pub use hexchat_callbacks::*;
pub use hexchat_entry_points::*;
pub use list_iterator::*;
pub use plugin::*;
pub use thread_facilities::*;
pub use user_data::*;
#[allow(unused_imports)]
pub use utils::*;

