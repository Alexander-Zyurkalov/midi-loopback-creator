mod midi_loopback;

use crate::midi_loopback::MIDILoopback;
use anyhow::{anyhow, Context, Error, Result};
use std::ffi::{c_void, CStr};
use std::os::raw::{c_char, c_int};
use std::ptr::{null, null_mut};

// ── Lua FFI declarations ────────────────────────────────────────────

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct lua_State {
    _private: [u8; 0],
}

#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
type lua_CFunction = *const unsafe extern "C" fn(L: *mut lua_State) -> c_int;

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct luaL_Reg {
    name: *const c_char,
    func: lua_CFunction,
}

#[allow(non_snake_case)]
unsafe extern "C" {
    fn lua_pushinteger(L: *mut lua_State, n: i64);
    fn lua_tointeger(L: *mut lua_State, index: c_int) -> i64;
    fn lua_pushnil(L: *mut lua_State);
    fn lua_pushvalue(L: *mut lua_State, index: c_int);
    fn lua_tolstring(L: *mut lua_State, index: c_int, len: *mut usize) -> *const c_char;
    fn lua_settable(L: *mut lua_State, index: c_int);
    fn lua_pushstring(L: *mut lua_State, s: *const c_char) -> *const c_char;
    fn lua_newuserdata(L: *mut lua_State, size: usize) -> *mut c_void;
    fn luaL_newmetatable(L: *mut lua_State, tname: *const c_char) -> c_int;
    fn lua_getfield(L: *mut lua_State, index: c_int, k: *const c_char);
    fn lua_setmetatable(L: *mut lua_State, objindex: c_int) -> c_int;
    fn lua_settop(L: *mut lua_State, index: c_int);
    fn lua_touserdata(L: *mut lua_State, index: c_int) -> *mut c_void;
    fn luaL_register(L: *mut lua_State, libname: *const c_char, l: *const luaL_Reg);
}

const LUA_REGISTRYINDEX: c_int = -10000;

const MIDI_LOOPBACK_MT_NAME: *const c_char = b"RustMIDILoopback\0".as_ptr() as *const c_char;

// ── Small helpers ───────────────────────────────────────────────────

#[inline]
#[allow(non_snake_case)]
unsafe fn lua_pop(L: *mut lua_State, n: c_int) {
    unsafe {
        lua_settop(L, -n - 1);
    }
}

#[allow(non_snake_case)]
unsafe fn make_lua_error(L: *mut lua_State, err: Error, nil_count: c_int) -> c_int {
    unsafe {
        for _ in 0..nil_count {
            lua_pushnil(L);
        }
        let err_cstring = std::ffi::CString::new(err.to_string()).unwrap_or_else(|_| {
            std::ffi::CString::new("Can't even make an error message".to_string()).unwrap()
        });
        lua_pushstring(L, err_cstring.as_ptr());
        nil_count + 1
    }
}

#[allow(non_snake_case)]
unsafe fn get_u8_or_error(L: *mut lua_State, argument_num: i32) -> Result<u8> {
    unsafe {
        let n = lua_tointeger(L, argument_num);
        if n < 1 || n > 255 {
            return Err(anyhow!(
                "Argument {} must be an integer between 1 and 255, got {}",
                argument_num,
                n
            ));
        }
        Ok(n as u8)
    }
}

#[allow(non_snake_case)]
unsafe fn get_string_or_error(L: *mut lua_State, argument_num: i32) -> Result<String> {
    unsafe {
        let c_str: *const c_char = lua_tolstring(L, argument_num, null_mut());
        if c_str.is_null() {
            return Err(anyhow!("Argument {} is not a string", argument_num));
        }
        let s = CStr::from_ptr(c_str)
            .to_str()
            .with_context(|| format!("Argument {} contains invalid UTF-8", argument_num))?
            .to_owned();
        Ok(s)
    }
}

#[allow(non_snake_case)]
unsafe fn get_loopback(L: *mut lua_State) -> Result<&'static mut MIDILoopback> {
    unsafe {
        let ud = lua_touserdata(L, 1) as *mut *mut MIDILoopback;
        if ud.is_null() || (*ud).is_null() {
            return Err(anyhow!("Invalid MIDILoopback userdata"));
        }
        Ok(&mut **ud)
    }
}

#[allow(non_snake_case)]
unsafe fn push_rust_string(L: *mut lua_State, s: &str) {
    unsafe {
        let cstring = std::ffi::CString::new(s)
            .unwrap_or_else(|_| std::ffi::CString::new("<invalid string>").unwrap());
        lua_pushstring(L, cstring.as_ptr());
    }
}

// ── Exported methods ────────────────────────────────────────────────

// ---------- new(name, instrument_id) -> userdata, unique_id, nil | nil, nil, err ----------

#[allow(non_snake_case)]
unsafe extern "C" fn new(L: *mut lua_State) -> c_int {
    unsafe {
        match new_inner(L) {
            Ok(_) => 3, // userdata + unique_id + nil
            Err(err) => make_lua_error(L, err, 2),
        }
    }
}

#[allow(non_snake_case)]
unsafe fn new_inner(L: *mut lua_State) -> Result<()> {
    unsafe {
        let name = get_string_or_error(L, 1)?;
        let instrument_id = get_u8_or_error(L, 2)?;

        let (loopback, unique_id) = MIDILoopback::new(&name, instrument_id)?;

        let boxed = Box::new(loopback);
        std::ptr::write(
            lua_newuserdata(L, size_of::<*mut MIDILoopback>()) as *mut *mut MIDILoopback,
            Box::into_raw(boxed),
        );
        lua_getfield(L, LUA_REGISTRYINDEX, MIDI_LOOPBACK_MT_NAME);
        lua_setmetatable(L, -2);
        lua_pushinteger(L, unique_id as i64);
        lua_pushnil(L);
        Ok(())
    }
}

// ---------- obj:rename(new_name) -> nil, err | nothing ----------

#[allow(non_snake_case)]
unsafe extern "C" fn rename(L: *mut lua_State) -> c_int {
    unsafe {
        match rename_inner(L) {
            Ok(()) => 0,
            Err(err) => make_lua_error(L, err, 1),
        }
    }
}

#[allow(non_snake_case)]
unsafe fn rename_inner(L: *mut lua_State) -> Result<()> {
    unsafe {
        let new_name = get_string_or_error(L, 2)?;
        let loopback = get_loopback(L)?;
        loopback.rename(&new_name)?; // just ? — already anyhow
        Ok(())
    }
}

// ---------- obj:get_name() -> string ----------

#[allow(non_snake_case)]
unsafe extern "C" fn get_name(L: *mut lua_State) -> c_int {
    unsafe {
        match get_name_inner(L) {
            Ok(_) => 1,
            Err(err) => make_lua_error(L, err, 1),
        }
    }
}

#[allow(non_snake_case)]
unsafe fn get_name_inner(L: *mut lua_State) -> Result<()> {
    unsafe {
        let loopback = get_loopback(L)?;
        push_rust_string(L, loopback.get_name());
        Ok(())
    }
}

// ---------- obj:get_unique_id() -> integer ----------

#[allow(non_snake_case)]
unsafe extern "C" fn get_unique_id(L: *mut lua_State) -> c_int {
    unsafe {
        match get_unique_id_inner(L) {
            Ok(_) => 1,
            Err(err) => make_lua_error(L, err, 1),
        }
    }
}

#[allow(non_snake_case)]
unsafe fn get_unique_id_inner(L: *mut lua_State) -> Result<()> {
    unsafe {
        let loopback = get_loopback(L)?;
        lua_pushinteger(L, loopback.get_unique_id() as i64);
        Ok(())
    }
}

// ---------- __gc ----------

#[allow(non_snake_case)]
unsafe extern "C" fn midi_loopback_gc(L: *mut lua_State) -> c_int {
    println!("MIDILoopback destructor was called");
    unsafe {
        let ud = lua_touserdata(L, 1) as *mut *mut MIDILoopback;
        if !ud.is_null() && !(*ud).is_null() {
            drop(Box::from_raw(*ud));
            *ud = null_mut();
        }
    }
    0
}

// ── Metatable registration ──────────────────────────────────────────

const MIDI_LOOPBACK_OBJECT_META: [luaL_Reg; 5] = [
    luaL_Reg {
        name: b"__gc\0".as_ptr() as *const c_char,
        func: midi_loopback_gc as lua_CFunction,
    },
    luaL_Reg {
        name: b"rename\0".as_ptr() as *const c_char,
        func: rename as lua_CFunction,
    },
    luaL_Reg {
        name: b"get_name\0".as_ptr() as *const c_char,
        func: get_name as lua_CFunction,
    },
    luaL_Reg {
        name: b"get_unique_id\0".as_ptr() as *const c_char,
        func: get_unique_id as lua_CFunction,
    },
    luaL_Reg {
        name: null(),
        func: null(),
    }, // sentinel
];

const MIDI_LOOPBACK_CLASS_META: [luaL_Reg; 2] = [
    luaL_Reg {
        name: b"new\0".as_ptr() as *const c_char,
        func: new as lua_CFunction,
    },
    luaL_Reg {
        name: null(),
        func: null(),
    }, // sentinel
];

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn luaopen_midi_loopback(L: *mut lua_State) -> c_int {
    unsafe {
        luaL_newmetatable(L, MIDI_LOOPBACK_MT_NAME);
        luaL_register(L, null(), MIDI_LOOPBACK_OBJECT_META.as_ptr());

        lua_pushstring(L, b"__index\0".as_ptr() as *const c_char);
        lua_pushvalue(L, -2);
        lua_settable(L, -3);

        lua_pop(L, 1);

        let library_name = b"midi_loopback\0".as_ptr() as *const c_char;
        luaL_register(L, library_name, MIDI_LOOPBACK_CLASS_META.as_ptr());
    }
    1
}
