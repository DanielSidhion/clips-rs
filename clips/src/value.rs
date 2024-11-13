use clips_sys::{CLIPSInstanceName, CLIPSSymbol};
use serde::{de::Visitor, Deserialize, Serialize};
use std::{
    ffi::{CStr, CString},
    fmt::Display,
};

use crate::{CLIPSFrom, CLIPSInto};

impl CLIPSFrom<usize> for clips_sys::CLIPSValue {
    fn from(value: usize, env: *mut clips_sys::Environment) -> clips_sys::CLIPSValue {
        let mut res = clips_sys::CLIPSValue::default();
        res.__bindgen_anon_1.integerValue = unsafe { clips_sys::CreateInteger(env, value as i64) };
        res
    }
}

impl CLIPSFrom<u64> for clips_sys::CLIPSValue {
    fn from(value: u64, env: *mut clips_sys::Environment) -> clips_sys::CLIPSValue {
        let mut res = clips_sys::CLIPSValue::default();
        res.__bindgen_anon_1.integerValue = unsafe { clips_sys::CreateInteger(env, value as i64) };
        res
    }
}

impl CLIPSFrom<i64> for clips_sys::CLIPSValue {
    fn from(value: i64, env: *mut clips_sys::Environment) -> clips_sys::CLIPSValue {
        let mut res = clips_sys::CLIPSValue::default();
        res.__bindgen_anon_1.integerValue = unsafe { clips_sys::CreateInteger(env, value) };
        res
    }
}

impl CLIPSFrom<String> for clips_sys::CLIPSValue {
    fn from(value: String, env: *mut clips_sys::Environment) -> clips_sys::CLIPSValue {
        let cstr = CString::new(value).unwrap();
        let mut res = clips_sys::CLIPSValue::default();
        res.__bindgen_anon_1.lexemeValue = unsafe { clips_sys::CreateString(env, cstr.as_ptr()) };
        res
    }
}

impl CLIPSFrom<bool> for clips_sys::CLIPSValue {
    fn from(value: bool, env: *mut clips_sys::Environment) -> clips_sys::CLIPSValue {
        let mut res = clips_sys::CLIPSValue::default();
        res.__bindgen_anon_1.lexemeValue = unsafe { clips_sys::CreateBoolean(env, value) };
        res
    }
}

impl CLIPSFrom<CLIPSSymbol> for clips_sys::CLIPSValue {
    fn from(value: CLIPSSymbol, env: *mut clips_sys::Environment) -> clips_sys::CLIPSValue {
        let cstr = CString::new(value.0).unwrap();
        let mut res = clips_sys::CLIPSValue::default();
        res.__bindgen_anon_1.lexemeValue = unsafe { clips_sys::CreateSymbol(env, cstr.as_ptr()) };
        res
    }
}

impl CLIPSFrom<CLIPSInstanceName> for clips_sys::CLIPSValue {
    fn from(value: CLIPSInstanceName, env: *mut clips_sys::Environment) -> clips_sys::CLIPSValue {
        let cstr = CString::new(value.0).unwrap();
        let mut res = clips_sys::CLIPSValue::default();
        res.__bindgen_anon_1.lexemeValue = unsafe { clips_sys::CreateSymbol(env, cstr.as_ptr()) };
        res
    }
}

impl CLIPSFrom<f64> for clips_sys::CLIPSValue {
    fn from(value: f64, env: *mut clips_sys::Environment) -> clips_sys::CLIPSValue {
        let mut res = clips_sys::CLIPSValue::default();
        res.__bindgen_anon_1.floatValue = unsafe { clips_sys::CreateFloat(env, value) };
        res
    }
}

impl CLIPSFrom<Vec<CLIPSValue>> for clips_sys::CLIPSValue {
    fn from(value: Vec<CLIPSValue>, env: *mut clips_sys::Environment) -> Self {
        let mut res = clips_sys::CLIPSValue::default();
        let builder = unsafe { clips_sys::CreateMultifieldBuilder(env, value.len()) };

        for val in value {
            let mut converted_value: clips_sys::CLIPSValue = CLIPSInto::into(val, env);
            unsafe {
                clips_sys::MBAppend(builder, &mut converted_value);
            }
        }

        res.__bindgen_anon_1.multifieldValue = unsafe { clips_sys::MBCreate(builder) };
        res
    }
}

impl CLIPSFrom<CLIPSValue> for clips_sys::CLIPSValue {
    fn from(value: CLIPSValue, env: *mut clips_sys::Environment) -> Self {
        match value {
            CLIPSValue::Int(v) => CLIPSInto::into(v, env),
            CLIPSValue::String(v) => CLIPSInto::into(v, env),
            CLIPSValue::Symbol(v) => CLIPSInto::into(CLIPSSymbol(v), env),
            CLIPSValue::Float(v) => CLIPSInto::into(v, env),
            CLIPSValue::Bool(v) => CLIPSInto::into(v, env),
            CLIPSValue::Multifield(v) => CLIPSInto::into(v, env),
        }
    }
}

// The Serialize impl is derived because we only ever want to serialise `CLIPSValue`s to JSON. To convert a CLIPSValue to CLIPS, we use the `CLIPSFrom` trait.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub enum CLIPSValue {
    Symbol(String),
    Int(i64),
    String(String),
    Float(f64),
    Bool(bool),
    Multifield(Vec<CLIPSValue>),
}

impl Display for CLIPSValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Symbol(val) => f.write_str(&val),
            Self::Int(val) => f.write_str(&val.to_string()),
            Self::String(val) => write!(f, "\"{}\"", val),
            Self::Float(val) => f.write_str(&val.to_string()),
            Self::Bool(val) => f.write_str(&val.to_string()),
            Self::Multifield(vals) => {
                f.write_str("(")?;

                for val in vals.iter() {
                    write!(f, "{}", val)?;
                }

                f.write_str(")")
            }
        }
    }
}

struct CLIPSValueVisitor {
    is_symbol: bool,
}

impl CLIPSValueVisitor {
    fn new() -> Self {
        Self { is_symbol: false }
    }
}

impl<'de> Deserialize<'de> for CLIPSValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(CLIPSValueVisitor::new())
    }
}

// Beware: this impl is written to work with both deserialisation from JSON and from CLIPS. Read it carefully to understand the entry points of each.
impl<'de> Visitor<'de> for CLIPSValueVisitor {
    type Value = CLIPSValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a CLIPS value")
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(CLIPSValue::Int(v as i64))
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(CLIPSValue::Int(v as i64))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(CLIPSValue::Int(v as i64))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(CLIPSValue::Int(v))
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(CLIPSValue::Int(v as i64))
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(CLIPSValue::Int(v as i64))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(CLIPSValue::Int(v as i64))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(CLIPSValue::Int(v as i64))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(CLIPSValue::Float(v as f64))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(CLIPSValue::Float(v))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(CLIPSValue::Bool(v))
    }

    fn visit_newtype_struct<D>(mut self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        self.is_symbol = true;
        deserializer.deserialize_identifier(self)
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if self.is_symbol {
            Ok(CLIPSValue::Symbol(v))
        } else {
            Ok(CLIPSValue::String(v))
        }
    }

    // Note: in theory this should only be called from `visit_map()` to deserialize from JSON. At the moment, there's no scenario where we'll try to directly deserialize a multifield from CLIPS.
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut vals = Vec::new();

        while let Some(val) = seq.next_element()? {
            vals.push(val);
        }

        Ok(CLIPSValue::Multifield(vals))
    }

    // In case we're deserializing from JSON, we'll fall here first, which is convenient because the CLIPS deserializer won't call it, so we can know we're in "JSON mode" and deal with it appropriately.
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut res = None;

        while let Some(key) = map.next_key::<String>()? {
            if res.is_some() {
                return Err(serde::de::Error::invalid_length(2, &"1"));
            }

            match key.as_str() {
                "Symbol" => {
                    res = Some(CLIPSValue::Symbol(map.next_value()?));
                }
                "Int" => {
                    res = Some(CLIPSValue::Int(map.next_value()?));
                }
                "String" => {
                    res = Some(CLIPSValue::String(map.next_value()?));
                }
                "Float" => {
                    res = Some(CLIPSValue::Float(map.next_value()?));
                }
                "Bool" => {
                    res = Some(CLIPSValue::Bool(map.next_value()?));
                }
                "Multifield" => {
                    res = Some(CLIPSValue::Multifield(map.next_value()?));
                }
                v => {
                    return Err(serde::de::Error::unknown_variant(
                        v,
                        &[
                            "Symbol",
                            "Int",
                            "UInt",
                            "String",
                            "Float",
                            "Bool",
                            "Multifield",
                        ],
                    ));
                }
            }
        }

        Ok(res.unwrap())
    }
}

pub(crate) fn extract_clipsvalue(val: clips_sys::CLIPSValue) -> CLIPSValue {
    let value_type = unsafe { (*val.__bindgen_anon_1.header).type_ } as u32;

    match value_type {
        clips_sys::FLOAT_TYPE => {
            CLIPSValue::Float(unsafe { (*val.__bindgen_anon_1.floatValue).contents })
        }
        clips_sys::INTEGER_TYPE => {
            CLIPSValue::Int(unsafe { (*val.__bindgen_anon_1.integerValue).contents })
        }
        clips_sys::SYMBOL_TYPE => {
            let symbol_val =
                unsafe { CStr::from_ptr((*val.__bindgen_anon_1.lexemeValue).contents) };
            let symbol_val = symbol_val.to_str().unwrap();

            match symbol_val {
                "TRUE" => CLIPSValue::Bool(true),
                "FALSE" => CLIPSValue::Bool(true),
                v => CLIPSValue::Symbol(v.to_string()),
            }
        }
        clips_sys::STRING_TYPE => CLIPSValue::String(unsafe {
            let cstr = CStr::from_ptr((*val.__bindgen_anon_1.lexemeValue).contents);
            cstr.to_str().unwrap().to_string()
        }),
        clips_sys::MULTIFIELD_TYPE => {
            let vals_len = unsafe { (*val.__bindgen_anon_1.multifieldValue).length };
            let mut vals = Vec::with_capacity(vals_len);

            for i in 0..vals_len {
                let curr_clipsvalue =
                    unsafe { (*val.__bindgen_anon_1.multifieldValue).contents[i] };
                vals.push(extract_clipsvalue(curr_clipsvalue));
            }

            CLIPSValue::Multifield(vals)
        }
        _ => unimplemented!(
            "Can't extract the value of a CLIPS value with type id '{}'.",
            value_type
        ),
    }
}
