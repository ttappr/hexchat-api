//! Mock Hexchat for unit tests and doc examples.
//!
//! Builds a fake `Hexchat` table backed by plain Rust stubs.
//!
//! Typical use:
//! ``` no_run
//! # use hexchat_api::test_utils::*;
//! let _g = test_guard();
//! let hc = create_mock_hexchat();
//! hc.print("hello");
//! assert_eq!(last_printed(), "hello");
//! ```

use libc::{c_char, c_int, c_void, time_t};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::{LazyLock, Mutex, MutexGuard};

use crate::hexchat::{
    C_AttrCallback,
    C_Callback,
    C_FDCallback,
    C_PrintCallback,
    C_TimerCallback,
    EventAttrs,
    Hexchat,
    hexchat_context,
    hexchat_hook,
    hexchat_list,
};

/// Serializes tests that mutate the mock globals.
pub static TEST_LOCK: Mutex<()> = Mutex::new(());

/// Strings recorded by the `c_print` stub.
pub static PRINT_LOG: Mutex<Vec<String>> =
    Mutex::new(Vec::new());

/// Commands recorded by the `c_command` stub.
pub static COMMAND_LOG: Mutex<Vec<String>> =
    Mutex::new(Vec::new());

/// Event names recorded by emit-print stubs.
pub static EMIT_LOG: Mutex<Vec<String>> =
    Mutex::new(Vec::new());

/// Strings owned by the mock so returned pointers
/// stay valid for the test run.
static OWNED: LazyLock<Mutex<Vec<CString>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Success flag for emit-print stubs. Default is 1.
#[derive(Debug, Clone, Copy)]
pub struct EmitDefault(pub c_int);

impl Default for EmitDefault {
    fn default() -> Self {
        EmitDefault(1)
    }
}

/// Configurable return values for the stubs.
#[derive(Debug)]
pub struct MockState {
    /// If set, `nickcmp` returns this.
    pub nickcmp_override: Option<c_int>,
    /// Map for `get_info(id)` lookups.
    pub info: HashMap<String, String>,
    /// Map for string prefs (type 1).
    pub prefs_str: HashMap<String, String>,
    /// Map for integer prefs (type 2).
    pub prefs_int: HashMap<String, c_int>,
    /// Map for bool prefs (type 3).
    pub prefs_bool: HashMap<String, bool>,
    /// Return value for emit-print stubs.
    pub emit_print_result: EmitDefault,
    /// If set, `strip` returns this.
    pub strip_override: Option<String>,
    /// Store for `pluginpref_set/get_str`.
    pub pluginprefs: HashMap<String, String>,
    /// Store for int plugin prefs.
    pub pluginprefs_int: HashMap<String, c_int>,
    /// When true, `find_context` is non-null.
    pub find_context_ok: bool,
    /// When true, `get_context` is non-null.
    pub get_context_ok: bool,
    /// Return value for `set_context`.
    pub set_context_result: c_int,
}

impl Default for MockState {
    fn default() -> Self {
        MockState {
            nickcmp_override: None,
            info: HashMap::new(),
            prefs_str: HashMap::new(),
            prefs_int: HashMap::new(),
            prefs_bool: HashMap::new(),
            emit_print_result: EmitDefault(1),
            strip_override: None,
            pluginprefs: HashMap::new(),
            pluginprefs_int: HashMap::new(),
            find_context_ok: false,
            get_context_ok: false,
            set_context_result: 1,
        }
    }
}

/// Global configurable mock state.
pub static MOCK: LazyLock<Mutex<MockState>> =
    LazyLock::new(|| Mutex::new(MockState::default()));

// ----------------------------------------------------------------
// Helpers.
// ----------------------------------------------------------------

fn cstr_to_string(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }
}

/// Keep `s` alive and return its pointer.
fn own_str(s: &str) -> *const c_char {
    let cs = CString::new(s).unwrap_or_default();
    let ptr = cs.as_ptr();
    OWNED.lock().unwrap().push(cs);
    ptr
}

/// Reset logs and mock state to defaults.
pub fn reset_mock() {
    PRINT_LOG.lock().unwrap().clear();
    COMMAND_LOG.lock().unwrap().clear();
    EMIT_LOG.lock().unwrap().clear();
    *MOCK.lock().unwrap() = MockState::default();
}

/// Last string passed to `print`, if any.
pub fn last_printed() -> String {
    PRINT_LOG
        .lock()
        .unwrap()
        .last()
        .cloned()
        .unwrap_or_default()
}

/// Last command passed to `command`, if any.
pub fn last_command() -> String {
    COMMAND_LOG
        .lock()
        .unwrap()
        .last()
        .cloned()
        .unwrap_or_default()
}

fn dummy_hook() -> *const hexchat_hook {
    0x1234usize as *const hexchat_hook
}

fn dummy_ctx() -> *const hexchat_context {
    0x2345usize as *const hexchat_context
}

// ----------------------------------------------------------------
// Native stubs.
// ----------------------------------------------------------------

unsafe extern "C" fn m_hook_command(_hp : *const Hexchat,
                                    _n  : *const c_char,
                                    _p  : c_int,
                                    _c  : C_Callback,
                                    _h  : *const c_char,
                                    _u  : *mut c_void)
    -> *const hexchat_hook
{
    dummy_hook()
}

unsafe extern "C" fn m_hook_server(_hp : *const Hexchat,
                                   _n  : *const c_char,
                                   _p  : c_int,
                                   _c  : C_Callback,
                                   _u  : *mut c_void)
    -> *const hexchat_hook
{
    dummy_hook()
}

unsafe extern "C" fn m_hook_print(_hp : *const Hexchat,
                                  _n  : *const c_char,
                                  _p  : c_int,
                                  _c  : C_PrintCallback,
                                  _u  : *mut c_void)
    -> *const hexchat_hook
{
    dummy_hook()
}

unsafe extern "C" fn m_hook_timer(_hp : *const Hexchat,
                                  _t  : c_int,
                                  _c  : C_TimerCallback,
                                  _u  : *mut c_void)
    -> *const hexchat_hook
{
    dummy_hook()
}

unsafe extern "C" fn m_hook_fd(_hp : *const Hexchat,
                               _f  : c_int,
                               _fl : c_int,
                               _c  : C_FDCallback,
                               _u  : *mut c_void)
    -> *const hexchat_hook
{
    dummy_hook()
}

unsafe extern "C" fn m_unhook(_hp : *const Hexchat,
                              _h  : *const hexchat_hook)
    -> *const c_void
{
    ptr::null::<c_void>()
}

unsafe extern "C" fn m_print(_hp  : *const Hexchat,
                             text : *const c_char)
{
    PRINT_LOG.lock().unwrap().push(cstr_to_string(text));
}

unsafe extern "C" fn m_printf_impl(_hp  : *const Hexchat,
                                   text : *const c_char)
{
    PRINT_LOG.lock().unwrap().push(cstr_to_string(text));
}

unsafe extern "C" fn m_command(_hp : *const Hexchat,
                               cmd : *const c_char)
{
    COMMAND_LOG.lock().unwrap().push(cstr_to_string(cmd));
}

unsafe extern "C" fn m_commandf_impl(_hp : *const Hexchat,
                                     cmd : *const c_char)
{
    COMMAND_LOG.lock().unwrap().push(cstr_to_string(cmd));
}

unsafe extern "C" fn m_nickcmp(_hp : *const Hexchat,
                               s1  : *const c_char,
                               s2  : *const c_char)
    -> c_int
{
    if let Some(v) = MOCK.lock().unwrap().nickcmp_override {
        return v;
    }
    let a = cstr_to_string(s1).to_lowercase();
    let b = cstr_to_string(s2).to_lowercase();
    if a == b {
        0
    } else if a < b {
        -1
    } else {
        1
    }
}

unsafe extern "C" fn m_set_context(_hp : *const Hexchat,
                                   _c  : *const hexchat_context)
    -> c_int
{
    MOCK.lock().unwrap().set_context_result
}

unsafe extern "C" fn m_find_context(_hp : *const Hexchat,
                                    _s  : *const c_char,
                                    _c  : *const c_char)
    -> *const hexchat_context
{
    if MOCK.lock().unwrap().find_context_ok {
        dummy_ctx()
    } else {
        ptr::null::<hexchat_context>()
    }
}

unsafe extern "C" fn m_get_context(_hp : *const Hexchat)
    -> *const hexchat_context
{
    if MOCK.lock().unwrap().get_context_ok {
        dummy_ctx()
    } else {
        ptr::null::<hexchat_context>()
    }
}

unsafe extern "C" fn m_get_info(_hp : *const Hexchat,
                                id  : *const c_char)
    -> *const c_char
{
    let key = cstr_to_string(id);
    let opt = MOCK.lock().unwrap().info.get(&key).cloned();
    if let Some(v) = opt {
        own_str(&v)
    } else {
        ptr::null::<c_char>()
    }
}

unsafe extern "C" fn m_get_prefs(_hp     : *const Hexchat,
                                 name    : *const c_char,
                                 string  : *mut *const c_char,
                                 integer : *mut c_int)
    -> c_int
{
    let key = cstr_to_string(name);
    let m = MOCK.lock().unwrap();
    if let Some(v) = m.prefs_str.get(&key).cloned() {
        drop(m);
        unsafe {
            *string = own_str(&v);
        }
        1
    } else if let Some(v) = m.prefs_int.get(&key).copied() {
        unsafe {
            *integer = v;
        }
        2
    } else if let Some(v) = m.prefs_bool.get(&key).copied() {
        unsafe {
            *integer = if v { 1 } else { 0 };
        }
        3
    } else {
        0
    }
}

unsafe extern "C" fn m_list_get(_hp : *const Hexchat,
                                _n  : *const c_char)
    -> *const hexchat_list
{
    ptr::null::<hexchat_list>()
}

unsafe extern "C" fn m_list_free(_hp : *const Hexchat,
                                 _l  : *const hexchat_list)
{
}

unsafe extern "C" fn m_list_fields(_hp : *const Hexchat,
                                   _n  : *const c_char)
    -> *const *const c_char
{
    ptr::null::<*const c_char>()
}

unsafe extern "C" fn m_list_next(_hp : *const Hexchat,
                                 _l  : *const hexchat_list)
    -> c_int
{
    0
}

unsafe extern "C" fn m_list_str(_hp : *const Hexchat,
                                _l  : *const hexchat_list,
                                _f  : *const c_char)
    -> *const c_char
{
    ptr::null::<c_char>()
}

unsafe extern "C" fn m_list_int(_hp : *const Hexchat,
                                _l  : *const hexchat_list,
                                _f  : *const c_char)
    -> c_int
{
    0
}

unsafe extern "C" fn m_plugingui_add(_hp : *const Hexchat,
                                     _f  : *const c_char,
                                     _n  : *const c_char,
                                     _d  : *const c_char,
                                     _v  : *const c_char,
                                     _r  : *const c_char)
    -> *const c_void
{
    0x3456usize as *const c_void
}

unsafe extern "C" fn m_plugingui_remove(_hp : *const Hexchat,
                                        _h  : *const c_void)
{
}

unsafe extern "C" fn m_emit_print_impl(_hp   : *const Hexchat,
                                       event : *const c_char)
    -> c_int
{
    EMIT_LOG.lock().unwrap().push(cstr_to_string(event));
    MOCK.lock().unwrap().emit_print_result.0
}

unsafe extern "C" fn m_read_fd(_hp : *const Hexchat,
                               _s  : *const c_void,
                               _b  : *mut c_char,
                               _l  : *mut c_int)
    -> c_int
{
    0
}

unsafe extern "C" fn m_list_time(_hp : *const Hexchat,
                                 _l  : *const hexchat_list,
                                 _n  : *const c_char)
    -> time_t
{
    0
}

unsafe extern "C" fn m_gettext(_hp : *const Hexchat,
                               msg : *const c_char)
    -> *const c_char
{
    msg
}

unsafe extern "C" fn m_send_modes(_hp : *const Hexchat,
                                  _t  : *const *const c_char,
                                  _n  : c_int,
                                  _m  : c_int,
                                  _s  : c_char,
                                  _mo : c_char)
{
}

unsafe extern "C" fn m_strip(_hp  : *const Hexchat,
                             s    : *const c_char,
                             _len : c_int,
                             _fl  : c_int)
    -> *const c_char
{
    let ov = MOCK.lock().unwrap().strip_override.clone();
    if let Some(v) = ov {
        own_str(&v)
    } else {
        own_str(&cstr_to_string(s))
    }
}

unsafe extern "C" fn m_free(_hp : *const Hexchat,
                            _p  : *const c_void)
{
    // Owned by OWNED; nothing to free.
}

unsafe extern "C" fn m_pluginpref_set_str(_hp : *const Hexchat,
                                          var : *const c_char,
                                          val : *const c_char)
    -> c_int
{
    let k = cstr_to_string(var);
    let v = cstr_to_string(val);
    MOCK.lock().unwrap().pluginprefs.insert(k, v);
    1
}

unsafe extern "C" fn m_pluginpref_get_str(_hp  : *const Hexchat,
                                          var  : *const c_char,
                                          dest : *mut c_char)
    -> c_int
{
    let key = cstr_to_string(var);
    let opt = MOCK.lock().unwrap().pluginprefs.get(&key).cloned();
    if let Some(v) = opt {
        let cs = CString::new(v).unwrap_or_default();
        let bytes = cs.as_bytes_with_nul();
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr() as *const c_char,
                dest,
                bytes.len(),
            );
        }
        1
    } else {
        0
    }
}

unsafe extern "C" fn m_pluginpref_set_int(_hp : *const Hexchat,
                                          var : *const c_char,
                                          val : c_int)
    -> c_int
{
    MOCK.lock()
        .unwrap()
        .pluginprefs_int
        .insert(cstr_to_string(var), val);
    1
}

unsafe extern "C" fn m_pluginpref_get_int(_hp : *const Hexchat,
                                          var : *const c_char)
    -> c_int
{
    let key = cstr_to_string(var);
    MOCK.lock()
        .unwrap()
        .pluginprefs_int
        .get(&key)
        .copied()
        .unwrap_or(0)
}

unsafe extern "C" fn m_pluginpref_delete(_hp : *const Hexchat,
                                         var : *const c_char)
    -> c_int
{
    let key = cstr_to_string(var);
    let mut m = MOCK.lock().unwrap();
    let a = m.pluginprefs.remove(&key).is_some();
    let b = m.pluginprefs_int.remove(&key).is_some();
    if a || b {
        1
    } else {
        0
    }
}

unsafe extern "C" fn m_pluginpref_list(_hp  : *const Hexchat,
                                       dest : *mut c_char)
    -> c_int
{
    let joined = {
        let m = MOCK.lock().unwrap();
        if m.pluginprefs.is_empty() {
            return 0;
        }
        let mut names: Vec<String> =
            m.pluginprefs.keys().cloned().collect();
        names.sort();
        names.join(",")
    };
    let cs = CString::new(joined).unwrap_or_default();
    let bytes = cs.as_bytes_with_nul();
    unsafe {
        ptr::copy_nonoverlapping(
            bytes.as_ptr() as *const c_char,
            dest,
            bytes.len(),
        );
    }
    1
}

unsafe extern "C" fn m_hook_server_attrs(_hp : *const Hexchat,
                                         _n  : *const c_char,
                                         _p  : c_int,
                                         _c  : C_AttrCallback,
                                         _u  : *const c_void)
    -> *const hexchat_hook
{
    dummy_hook()
}

unsafe extern "C" fn m_hook_print_attrs(_hp : *const Hexchat,
                                        _n  : *const c_char,
                                        _p  : c_int,
                                        _c  : C_AttrCallback,
                                        _u  : *const c_void)
    -> *const hexchat_hook
{
    dummy_hook()
}

unsafe extern "C" fn m_emit_print_attrs_impl(_hp   : *const Hexchat,
                                             _a    : *const EventAttrs,
                                             event : *const c_char)
    -> c_int
{
    EMIT_LOG.lock().unwrap().push(cstr_to_string(event));
    MOCK.lock().unwrap().emit_print_result.0
}

unsafe extern "C" fn m_event_attrs_create(_hp : *const Hexchat)
    -> *mut EventAttrs
{
    Box::into_raw(Box::new(EventAttrs {
        server_time_utc: 0,
    }))
}

unsafe extern "C" fn m_event_attrs_free(_hp   : *const Hexchat,
                                        attrs : *mut EventAttrs)
{
    if !attrs.is_null() {
        unsafe {
            drop(Box::from_raw(attrs));
        }
    }
}

// ----------------------------------------------------------------
// Construction.
// ----------------------------------------------------------------

/// Cast a fixed-arg stub to a C-variadic fn pointer.
///
/// The mock never reads varargs, so ignoring the extra
/// args is sound for the test ABI.
unsafe fn to_variadic2<R>(f: unsafe extern "C" fn(*const Hexchat, *const c_char) -> R) 
    -> unsafe extern "C" fn(*const Hexchat, *const c_char, ...) -> R 
{
    core::mem::transmute(f)
}

unsafe fn to_variadic3<R>(f: unsafe extern "C" fn(*const Hexchat, *const EventAttrs, *const c_char) -> R) 
    -> unsafe extern "C" fn(*const Hexchat, *const EventAttrs, *const c_char, ...) -> R 
{
    core::mem::transmute(f)
}

unsafe fn to_variadic_void(f: unsafe extern "C" fn(*const Hexchat, *const c_char)) 
    -> unsafe extern "C" fn(*const Hexchat, *const c_char, ...) 
{
    core::mem::transmute(f)
}

/// Owned default table. Built once; never mutated.
fn default_hexchat() -> Hexchat {
    Hexchat {
        c_hook_command: m_hook_command,
        c_hook_server: m_hook_server,
        c_hook_print: m_hook_print,
        c_hook_timer: m_hook_timer,
        c_hook_fd: m_hook_fd,
        c_unhook: m_unhook,
        c_print: m_print,
        c_printf: unsafe { to_variadic_void(m_printf_impl) },
        c_command: m_command,
        c_commandf: unsafe { to_variadic_void(m_commandf_impl) },
        c_nickcmp: m_nickcmp,
        c_set_context: m_set_context,
        c_find_context: m_find_context,
        c_get_context: m_get_context,
        c_get_info: m_get_info,
        c_get_prefs: m_get_prefs,
        c_list_get: m_list_get,
        c_list_free: m_list_free,
        c_list_fields: m_list_fields,
        c_list_next: m_list_next,
        c_list_str: m_list_str,
        c_list_int: m_list_int,
        c_plugingui_add: m_plugingui_add,
        c_plugingui_remove: m_plugingui_remove,
        c_emit_print: unsafe { to_variadic2(m_emit_print_impl) },
        c_read_fd: m_read_fd,
        c_list_time: m_list_time,
        c_gettext: m_gettext,
        c_send_modes: m_send_modes,
        c_strip: m_strip,
        c_free: m_free,
        c_pluginpref_set_str: m_pluginpref_set_str,
        c_pluginpref_get_str: m_pluginpref_get_str,
        c_pluginpref_set_int: m_pluginpref_set_int,
        c_pluginpref_get_int: m_pluginpref_get_int,
        c_pluginpref_delete: m_pluginpref_delete,
        c_pluginpref_list: m_pluginpref_list,
        c_hook_server_attrs: m_hook_server_attrs,
        c_hook_print_attrs: m_hook_print_attrs,
        c_emit_print_attrs: unsafe { to_variadic3(m_emit_print_attrs_impl) },
        c_event_attrs_create: m_event_attrs_create,
        c_event_attrs_free: m_event_attrs_free,
    }
}

/// Shared mock table. Fn pointers never change; per-test behavior lives in 
/// `MOCK` and the logs.
/// 
static MOCK_HEXCHAT: LazyLock<Hexchat> = LazyLock::new(default_hexchat);

/// Return the shared mock table.
///
/// Same `&'static` each call; no allocation. Use `reset_mock()` (or 
/// `test_guard()`) for fresh behavior defaults.
/// 
pub fn get_mock_hexchat() -> &'static Hexchat {
    &MOCK_HEXCHAT
}

/// Guard installing the mock as global `PHEXCHAT`.
///
/// Hold it for the whole test. Serializes via `TEST_LOCK`.
/// 
pub struct TestGuard {
    _lock: MutexGuard<'static, ()>,
    old_hc: *const Hexchat,
    #[cfg(feature = "threadsafe")]
    old_main: Option<std::thread::ThreadId>,
}

impl TestGuard {
    fn new() -> Self {
        let lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: under TEST_LOCK so no other test
        // touches these globals concurrently.
        let old_hc = unsafe { crate::PHEXCHAT };
        #[cfg(feature = "threadsafe")]
        let old_main = unsafe { crate::MAIN_THREAD_ID };
        let hc = get_mock_hexchat();
        unsafe {
            crate::PHEXCHAT = hc as *const Hexchat;
        }
        #[cfg(feature = "threadsafe")]
        unsafe {
            crate::MAIN_THREAD_ID =
                Some(std::thread::current().id());
        }
        crate::hook::Hook::init();
        reset_mock();
        TestGuard {
            _lock: lock,
            old_hc,
            #[cfg(feature = "threadsafe")]
            old_main,
        }
    }
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        crate::hook::Hook::deinit();
        unsafe {
            crate::PHEXCHAT = self.old_hc;
        }
        #[cfg(feature = "threadsafe")]
        unsafe {
            crate::MAIN_THREAD_ID = self.old_main;
        }
    }
}

/// Acquire the serial guard and install a mock.
pub fn test_guard() -> TestGuard {
    TestGuard::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hexchat::StripFlags;

    #[test]
    fn mock_hexchat_is_shared_static() {
        let _g = test_guard();
        let a = get_mock_hexchat() as *const Hexchat;
        let b = get_mock_hexchat() as *const Hexchat;
        println!("\n\nshared static\n");
        assert!(std::ptr::eq(a, b));
        println!("ok\n");
    }

    #[test]
    fn print_and_command_are_recorded() {
        let _g = test_guard();
        let hc = get_mock_hexchat();
        println!("\n\nprint/command recording\n");
        hc.print("hello");
        hc.command("SAY hi");
        assert_eq!(last_printed(), "hello");
        assert_eq!(last_command(), "SAY hi");
        assert_eq!(
            PRINT_LOG.lock().unwrap().as_slice(),
            ["hello"]
        );
        println!("ok\n");
    }

    #[test]
    fn nickcmp_compares_and_overrides() {
        let _g = test_guard();
        let hc = get_mock_hexchat();
        println!("\n\nnickcmp\n");
        assert_eq!(hc.nickcmp("Nick", "nick"), 0);
        assert!(hc.nickcmp("a", "b") < 0);
        MOCK.lock().unwrap().nickcmp_override = Some(42);
        assert_eq!(hc.nickcmp("a", "b"), 42);
        println!("ok\n");
    }

    #[test]
    fn get_info_returns_configured_values() {
        let _g = test_guard();
        let hc = get_mock_hexchat();
        println!("\n\nget_info\n");
        MOCK.lock()
            .unwrap()
            .info
            .insert("channel".into(), "#rust".into());
        assert_eq!(
            hc.get_info("channel").as_deref(),
            Some("#rust")
        );
        assert_eq!(hc.get_info("missing"), None);
        println!("ok\n");
    }

    #[test]
    fn get_prefs_covers_all_types() {
        let _g = test_guard();
        let hc = get_mock_hexchat();
        println!("\n\nget_prefs\n");
        {
            let mut m = MOCK.lock().unwrap();
            m.prefs_str.insert("s".into(), "v".into());
            m.prefs_int.insert("i".into(), 7);
            m.prefs_bool.insert("b".into(), true);
        }
        assert!(matches!(
            hc.get_prefs("s"),
            Some(crate::hexchat::PrefValue::StringVal(_))
        ));
        assert!(matches!(
            hc.get_prefs("i"),
            Some(crate::hexchat::PrefValue::IntegerVal(7))
        ));
        assert!(matches!(
            hc.get_prefs("b"),
            Some(crate::hexchat::PrefValue::BoolVal(true))
        ));
        assert!(hc.get_prefs("nope").is_none());
        println!("ok\n");
    }

    #[test]
    fn strip_and_emit_and_pluginprefs() {
        let _g = test_guard();
        let hc = get_mock_hexchat();
        println!("\n\nstrip/emit/prefs\n");
        let s = hc.strip("hi", StripFlags::StripBoth);
        assert_eq!(s.as_deref(), Some("hi"));
        MOCK.lock().unwrap().strip_override = Some("plain".into());
        let s = hc.strip("x", StripFlags::StripBoth);
        assert_eq!(s.as_deref(), Some("plain"));

        hc.emit_print("Message", &["a"]).unwrap();
        assert_eq!(EMIT_LOG.lock().unwrap().as_slice(),
                   ["Message"]);
        MOCK.lock().unwrap().emit_print_result = EmitDefault(0);
        assert!(hc.emit_print("X", &[]).is_err());

        use crate::hexchat::PrefValue::*;
        assert!(hc.pluginpref_set(
            "k",
            StringVal("v".into())
        ));
        let got = hc.pluginpref_get("k").unwrap();
        assert_eq!(format!("{got:?}"), "StringVal(\"sv\")");
        assert_eq!(hc.pluginpref_list().unwrap(),
                   vec!["k".to_string()]);
        println!("ok\n");
    }
}
