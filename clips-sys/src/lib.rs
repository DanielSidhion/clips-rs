#![allow(non_snake_case)]
mod sys;
pub use sys::*;

use std::{
    ffi::CStr,
    ptr::{addr_of, addr_of_mut},
};
use thiserror::Error;

// bindgen loses these macros, so we're redefining them directly here as functions.

pub unsafe fn GetEnvironmentData(
    env: *mut Environment,
    id: ::std::os::raw::c_uint,
) -> *mut ::std::os::raw::c_void {
    let env_struct = env.as_ref().unwrap();
    let location = addr_of!(*env_struct.theData).offset(id.try_into().unwrap());
    let data_ptr = ::std::ptr::read_unaligned(location) as *mut *mut ::std::os::raw::c_void;
    ::std::ptr::read_unaligned(data_ptr)
}

pub unsafe fn SetEnvironmentData(
    env: *mut Environment,
    id: ::std::os::raw::c_uint,
    value: *mut ::std::os::raw::c_void,
) {
    let env_struct = env.as_ref().unwrap();
    let location = addr_of_mut!(*env_struct.theData).offset(id.try_into().unwrap());
    let data_ptr = ::std::ptr::read_unaligned(location) as *mut *mut ::std::os::raw::c_void;
    ::std::ptr::write_unaligned(data_ptr, value);
}

#[derive(Error, Debug)]
pub enum UDFConversionError {
    #[error("tried to convert an UDF value with type {} into another type", .0)]
    InvalidType(&'static str),
    #[error("the string value of the UDF value isn't valid unicode")]
    ValueNotUnicode,
    #[error("the UDF value given is a symbol, but doesn't correspond to the boolean symbols used by CLIPS")]
    ValueNotBoolean,
}

// TODO: do this for more types.
pub struct CLIPSSymbol(pub String);
pub struct CLIPSInstanceName(pub String);

impl TryFrom<sys::UDFValue> for usize {
    type Error = UDFConversionError;

    fn try_from(value: sys::UDFValue) -> Result<Self, Self::Error> {
        let type_num = unsafe { (*value.__bindgen_anon_1.header).type_ } as u32;

        if type_num == sys::INTEGER_TYPE {
            Ok(unsafe { (*value.__bindgen_anon_1.integerValue).contents } as usize)
        } else {
            Err(UDFConversionError::InvalidType("integer"))
        }
    }
}

impl TryFrom<sys::UDFValue> for u64 {
    type Error = UDFConversionError;

    fn try_from(value: sys::UDFValue) -> Result<Self, Self::Error> {
        let type_num = unsafe { (*value.__bindgen_anon_1.header).type_ } as u32;

        if type_num == sys::INTEGER_TYPE {
            Ok(unsafe { (*value.__bindgen_anon_1.integerValue).contents } as u64)
        } else {
            Err(UDFConversionError::InvalidType("integer"))
        }
    }
}

impl TryFrom<sys::UDFValue> for String {
    type Error = UDFConversionError;

    fn try_from(value: sys::UDFValue) -> Result<Self, Self::Error> {
        let type_num = unsafe { (*value.__bindgen_anon_1.header).type_ } as u32;

        if type_num == sys::STRING_TYPE {
            let c_str = unsafe { CStr::from_ptr((*value.__bindgen_anon_1.lexemeValue).contents) };
            Ok(c_str
                .to_str()
                .map_err(|_| UDFConversionError::ValueNotUnicode)?
                .to_string())
        } else {
            Err(UDFConversionError::InvalidType("string"))
        }
    }
}

impl TryFrom<sys::UDFValue> for CLIPSSymbol {
    type Error = UDFConversionError;

    fn try_from(value: sys::UDFValue) -> Result<Self, Self::Error> {
        let type_num = unsafe { (*value.__bindgen_anon_1.header).type_ } as u32;

        if type_num == sys::SYMBOL_TYPE {
            let c_str = unsafe { CStr::from_ptr((*value.__bindgen_anon_1.lexemeValue).contents) };
            Ok(CLIPSSymbol(
                c_str
                    .to_str()
                    .map_err(|_| UDFConversionError::ValueNotUnicode)?
                    .to_string(),
            ))
        } else {
            Err(UDFConversionError::InvalidType("symbol"))
        }
    }
}

impl TryFrom<sys::UDFValue> for bool {
    type Error = UDFConversionError;

    fn try_from(value: sys::UDFValue) -> Result<Self, Self::Error> {
        let type_num = unsafe { (*value.__bindgen_anon_1.header).type_ } as u32;

        if type_num == sys::SYMBOL_TYPE {
            let c_str = unsafe { CStr::from_ptr((*value.__bindgen_anon_1.lexemeValue).contents) };

            match c_str.to_str().unwrap() {
                "TRUE" => Ok(true),
                "FALSE" => Ok(false),
                _ => Err(UDFConversionError::ValueNotBoolean),
            }
        } else {
            Err(UDFConversionError::InvalidType("bool (symbol)"))
        }
    }
}

impl TryFrom<sys::UDFValue> for CLIPSInstanceName {
    type Error = UDFConversionError;

    fn try_from(value: sys::UDFValue) -> Result<Self, Self::Error> {
        let type_num = unsafe { (*value.__bindgen_anon_1.header).type_ } as u32;

        if type_num == sys::INSTANCE_NAME_TYPE {
            let c_str = unsafe { CStr::from_ptr((*value.__bindgen_anon_1.lexemeValue).contents) };
            Ok(CLIPSInstanceName(
                c_str
                    .to_str()
                    .map_err(|_| UDFConversionError::ValueNotUnicode)?
                    .to_string(),
            ))
        } else {
            Err(UDFConversionError::InvalidType("symbol"))
        }
    }
}

impl TryFrom<sys::UDFValue> for f64 {
    type Error = UDFConversionError;

    fn try_from(value: sys::UDFValue) -> Result<Self, Self::Error> {
        let type_num = unsafe { (*value.__bindgen_anon_1.header).type_ } as u32;

        if type_num == sys::FLOAT_TYPE {
            Ok(unsafe { (*value.__bindgen_anon_1.floatValue).contents })
        } else {
            Err(UDFConversionError::InvalidType("float"))
        }
    }
}
