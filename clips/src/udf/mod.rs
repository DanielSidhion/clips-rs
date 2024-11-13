pub mod conversion;
use std::{collections::HashMap, sync::OnceLock};

use crate::{CLIPSEnvironment, CLIPSError, CLIPSInto, CLIPSResult};

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct UDFType: u32 {
        const Boolean = clips_sys::CLIPSType_BOOLEAN_BIT;
        const Float = clips_sys::CLIPSType_FLOAT_BIT;
        const ExternalAddress = clips_sys::CLIPSType_EXTERNAL_ADDRESS_BIT;
        const FactAddress = clips_sys::CLIPSType_FACT_ADDRESS_BIT;
        const InstanceAddress = clips_sys::CLIPSType_INSTANCE_ADDRESS_BIT;
        const Integer = clips_sys::CLIPSType_INTEGER_BIT;
        const Multifield = clips_sys::CLIPSType_MULTIFIELD_BIT;
        const InstanceName = clips_sys::CLIPSType_INSTANCE_NAME_BIT;
        const String = clips_sys::CLIPSType_STRING_BIT;
        const Symbol = clips_sys::CLIPSType_SYMBOL_BIT;
        const Void = clips_sys::CLIPSType_VOID_BIT;

        const Number = Self::Float.bits() | Self::Integer.bits();
        const Lexeme = Self::Symbol.bits() | Self::String.bits();
        const Address = Self::ExternalAddress.bits() | Self::FactAddress.bits() | Self::InstanceAddress.bits();
        const Instance = Self::InstanceAddress.bits() | Self::InstanceName.bits();
        const Singlefield = Self::Number.bits() | Self::Lexeme.bits() | Self::Address.bits() | Self::Instance.bits();
        const Any = Self::Void.bits() | Self::Singlefield.bits() | Self::Multifield.bits();
    }
}

impl UDFType {
    pub fn as_character_code(&self) -> String {
        if self.contains(Self::Any) {
            return "*".to_string();
        }

        static CHARACTER_CODE_MAP: OnceLock<HashMap<UDFType, char>> = OnceLock::new();
        let character_map = CHARACTER_CODE_MAP.get_or_init(|| {
            HashMap::from([
                (Self::Boolean, 'b'),
                (Self::Float, 'd'),
                (Self::ExternalAddress, 'e'),
                (Self::FactAddress, 'f'),
                (Self::InstanceAddress, 'i'),
                (Self::Integer, 'l'),
                (Self::Multifield, 'm'),
                (Self::InstanceName, 'n'),
                (Self::String, 's'),
                (Self::Symbol, 'y'),
                (Self::Void, 'v'),
            ])
        });

        let mut res = String::with_capacity(11);

        for (bit, char_code) in character_map.iter() {
            if self.contains(*bit) {
                res.push(*char_code);
            }
        }

        res.shrink_to_fit();
        res
    }
}

pub struct UDFData {
    env: *mut clips_sys::Environment,
    context: *mut clips_sys::UDFContext,
    result: *mut clips_sys::UDFValue,
}

impl UDFData {
    pub fn new(
        env: *mut clips_sys::Environment,
        context: *mut clips_sys::UDFContext,
        result: *mut clips_sys::UDFValue,
    ) -> Self {
        Self {
            env,
            context,
            result,
        }
    }

    pub fn env(&self) -> CLIPSEnvironment {
        CLIPSEnvironment::from_raw(self.env)
    }

    pub fn num_args(&self) -> usize {
        let res = unsafe { clips_sys::UDFArgumentCount(self.context) } as usize;
        res
    }

    pub fn first_arg<T>(&self) -> CLIPSResult<T>
    where
        T: std::convert::TryFrom<clips_sys::UDFValue>,
        CLIPSError: From<<T as TryFrom<clips_sys::UDFValue>>::Error>,
    {
        let mut arg = clips_sys::UDFValue::default();

        let res =
            unsafe { clips_sys::UDFFirstArgument(self.context, UDFType::Any.bits(), &mut arg) };

        if !res {
            Err(CLIPSError::ArgumentNotRetrieved)
        } else {
            Ok(arg.try_into()?)
        }
    }

    pub fn next_arg<T>(&self) -> CLIPSResult<T>
    where
        T: std::convert::TryFrom<clips_sys::UDFValue>,
        CLIPSError: From<<T as TryFrom<clips_sys::UDFValue>>::Error>,
    {
        let mut arg = clips_sys::UDFValue::default();

        let res =
            unsafe { clips_sys::UDFNextArgument(self.context, UDFType::Any.bits(), &mut arg) };

        if !res {
            Err(CLIPSError::ArgumentNotRetrieved)
        } else {
            Ok(arg.try_into()?)
        }
    }

    pub fn nth_arg<T>(&self, n: u32) -> CLIPSResult<T>
    where
        T: std::convert::TryFrom<clips_sys::UDFValue>,
        CLIPSError: From<<T as TryFrom<clips_sys::UDFValue>>::Error>,
    {
        let mut arg = clips_sys::UDFValue::default();

        let res =
            unsafe { clips_sys::UDFNthArgument(self.context, n, UDFType::Any.bits(), &mut arg) };

        if !res {
            Err(CLIPSError::ArgumentNotRetrieved)
        } else {
            Ok(arg.try_into()?)
        }
    }

    pub fn set_result<T>(&mut self, res: T) -> CLIPSResult<()>
    where
        T: CLIPSInto<clips_sys::UDFValue>,
    {
        let converted_value: clips_sys::UDFValue = res.into(self.env);
        unsafe {
            // `converted_value` will be dropped when `set_result` finishes running, but the pointer we care about will still be captured by `self.result`.
            (*self.result).__bindgen_anon_1 = converted_value.__bindgen_anon_1;
        }

        Ok(())
    }

    pub fn throw_error(&self) -> CLIPSResult<()> {
        unsafe {
            clips_sys::UDFThrowError(self.context);
        }

        Ok(())
    }
}
