
//! This file is used to organize the namespace structure of the crate that
//! the user writing a plugin will see. The approach taken is to put all the
//! functions and structs in a flat namespace, `hexchat_api::`, to keep things
//! simple for the user.

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

