#![allow(dead_code)]

use libc::time_t;
use std::ffi::c_void;

use crate::Context;

//#[derive(Debug, Deserialize, Serialize)]
pub enum HexchatValue {
    StringValue(String),
    IntegerValue(i32),
    BoolValue(bool),
    PointerValue(*const c_void),
    ContextValue(Context),
    TimeValue(time_t),
}


