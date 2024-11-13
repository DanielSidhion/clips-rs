use clips_sys::{CLIPSInstanceName, CLIPSSymbol};
use std::ffi::CString;

use crate::CLIPSFrom;

impl CLIPSFrom<usize> for clips_sys::UDFValue {
    fn from(value: usize, env: *mut clips_sys::Environment) -> clips_sys::UDFValue {
        let mut res = clips_sys::UDFValue::default();
        res.__bindgen_anon_1.integerValue = unsafe { clips_sys::CreateInteger(env, value as i64) };
        res
    }
}

impl CLIPSFrom<u64> for clips_sys::UDFValue {
    fn from(value: u64, env: *mut clips_sys::Environment) -> clips_sys::UDFValue {
        let mut res = clips_sys::UDFValue::default();
        res.__bindgen_anon_1.integerValue = unsafe { clips_sys::CreateInteger(env, value as i64) };
        res
    }
}

impl CLIPSFrom<String> for clips_sys::UDFValue {
    fn from(value: String, env: *mut clips_sys::Environment) -> clips_sys::UDFValue {
        let cstr = CString::new(value).unwrap();
        let mut res = clips_sys::UDFValue::default();
        res.__bindgen_anon_1.lexemeValue = unsafe { clips_sys::CreateString(env, cstr.as_ptr()) };
        res
    }
}

impl CLIPSFrom<bool> for clips_sys::UDFValue {
    fn from(value: bool, env: *mut clips_sys::Environment) -> clips_sys::UDFValue {
        let mut res = clips_sys::UDFValue::default();
        res.__bindgen_anon_1.lexemeValue = unsafe { clips_sys::CreateBoolean(env, value) };
        res
    }
}

impl CLIPSFrom<CLIPSSymbol> for clips_sys::UDFValue {
    fn from(value: CLIPSSymbol, env: *mut clips_sys::Environment) -> clips_sys::UDFValue {
        let cstr = CString::new(value.0).unwrap();
        let mut res = clips_sys::UDFValue::default();
        res.__bindgen_anon_1.lexemeValue = unsafe { clips_sys::CreateSymbol(env, cstr.as_ptr()) };
        res
    }
}

impl CLIPSFrom<CLIPSInstanceName> for clips_sys::UDFValue {
    fn from(value: CLIPSInstanceName, env: *mut clips_sys::Environment) -> clips_sys::UDFValue {
        let cstr = CString::new(value.0).unwrap();
        let mut res = clips_sys::UDFValue::default();
        res.__bindgen_anon_1.lexemeValue = unsafe { clips_sys::CreateSymbol(env, cstr.as_ptr()) };
        res
    }
}

impl CLIPSFrom<f64> for clips_sys::UDFValue {
    fn from(value: f64, env: *mut clips_sys::Environment) -> clips_sys::UDFValue {
        let mut res = clips_sys::UDFValue::default();
        res.__bindgen_anon_1.floatValue = unsafe { clips_sys::CreateFloat(env, value) };
        res
    }
}
