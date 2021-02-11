#![allow(unused_variables, unused_imports)]

//! This is an example for how a plugin can be written using the hexchat_plugin
//! library. The library is still in early development, but the plan is to
//! have it as a static lib that can be linked in to plugin projects similar
//! to this example.

/* EXAMPLE PLUGIN USING RUST HEXCHAT API */

// Note: The module files have to be listed in lib.rs for the compiler to
//       locate them. Otherwise hexchat.rs can't access hook.rs for example.


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
mod utils;

pub use hook::*;
pub use callback_data::*;
pub use consts::*;
pub use context::*;
pub use hexchat::*;
pub use hexchat_callbacks::*;
pub use hexchat_entry_points::*;
pub use list_iterator::*;
pub use plugin::*;
pub use thread_facilities::*;
pub use utils::*;

