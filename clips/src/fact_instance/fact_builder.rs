use std::ffi::CString;

use clips_sys::CLIPSValue;

use crate::{
    translate_put_slot_error, CLIPSError, CLIPSInto, CLIPSResult, FactOrInstanceBuilderData,
};

pub(crate) struct CLIPSFactBuilder {
    pub(crate) fb: *mut clips_sys::FactBuilder,
}

pub struct FactBuilderData {
    fb: *mut clips_sys::FactBuilder,
    env: *mut clips_sys::environmentData,
}

impl FactBuilderData {
    pub(crate) fn new(
        fb: *mut clips_sys::FactBuilder,
        env: *mut clips_sys::environmentData,
    ) -> Self {
        Self { fb, env }
    }

    pub(crate) fn assert(self) -> CLIPSResult<()> {
        let res = unsafe { clips_sys::FBAssert(self.fb) };

        if res.is_null() {
            let res = unsafe { clips_sys::FBError(self.env) };

            match res {
                clips_sys::FactBuilderError_FBE_NULL_POINTER_ERROR => unreachable!(), // Due to the trait, we already have a template name so this error can't happen.
                clips_sys::FactBuilderError_FBE_COULD_NOT_ASSERT_ERROR => {
                    Err(CLIPSError::UnableToAssertFact)
                }
                clips_sys::FactBuilderError_FBE_RULE_NETWORK_ERROR => Err(CLIPSError::RuleNetwork),
                _ => unreachable!(),
            }
        } else {
            Ok(())
        }
    }
}

impl FactOrInstanceBuilderData for FactBuilderData {
    fn put_slot<T: CLIPSInto<CLIPSValue>>(&self, slot_name: &str, val: T) -> CLIPSResult<()> {
        let slot_name = CString::new(slot_name).unwrap();
        // Must keep the reference until we're done with this function call.
        let mut slot_value = val.into(self.env);
        let slot_value_raw: *mut CLIPSValue = &mut slot_value;

        translate_put_slot_error(unsafe {
            clips_sys::FBPutSlot(self.fb, slot_name.as_ptr(), slot_value_raw)
        })
    }

    fn put_int_slot<T: Into<i64>>(&self, slot_name: &str, val: T) -> CLIPSResult<()> {
        let slot_name = CString::new(slot_name).unwrap();

        translate_put_slot_error(unsafe {
            clips_sys::FBPutSlotInteger(self.fb, slot_name.as_ptr(), val.into())
        })
    }

    fn put_float_slot<T: Into<f64>>(&self, slot_name: &str, val: T) -> CLIPSResult<()> {
        let slot_name = CString::new(slot_name).unwrap();

        translate_put_slot_error(unsafe {
            clips_sys::FBPutSlotFloat(self.fb, slot_name.as_ptr(), val.into())
        })
    }

    fn put_instance_name_slot<T: Into<Vec<u8>>>(&self, slot_name: &str, val: T) -> CLIPSResult<()> {
        let slot_name = CString::new(slot_name).unwrap();
        let slot_value = CString::new(val).unwrap();

        translate_put_slot_error(unsafe {
            clips_sys::FBPutSlotInstanceName(self.fb, slot_name.as_ptr(), slot_value.as_ptr())
        })
    }

    fn put_symbol_slot<T: Into<Vec<u8>>>(&self, slot_name: &str, val: T) -> CLIPSResult<()> {
        let slot_name = CString::new(slot_name).unwrap();
        let slot_value = CString::new(val).unwrap();

        translate_put_slot_error(unsafe {
            clips_sys::FBPutSlotSymbol(self.fb, slot_name.as_ptr(), slot_value.as_ptr())
        })
    }

    fn put_string_slot<T: Into<Vec<u8>>>(&self, slot_name: &str, val: T) -> CLIPSResult<()> {
        let slot_name = CString::new(slot_name).unwrap();
        let slot_value = CString::new(val).unwrap();

        translate_put_slot_error(unsafe {
            clips_sys::FBPutSlotString(self.fb, slot_name.as_ptr(), slot_value.as_ptr())
        })
    }

    fn put_multifield_slot<T: CLIPSInto<CLIPSValue>>(
        &self,
        slot_name: &str,
        vals: Vec<T>,
    ) -> CLIPSResult<()> {
        let slot_name = CString::new(slot_name).unwrap();

        let mb = unsafe { clips_sys::CreateMultifieldBuilder(self.env, vals.len()) };

        for val in vals {
            let mut clips_val = val.into(self.env);
            let clips_val_raw: *mut CLIPSValue = &mut clips_val;
            unsafe { clips_sys::MBAppend(mb, clips_val_raw) };
        }

        let multifield = unsafe { clips_sys::MBCreate(mb) };
        unsafe { clips_sys::MBDispose(mb) };

        translate_put_slot_error(unsafe {
            clips_sys::FBPutSlotMultifield(self.fb, slot_name.as_ptr(), multifield)
        })
    }
}
