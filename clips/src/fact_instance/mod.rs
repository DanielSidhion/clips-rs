use clips_sys::CLIPSValue;

use crate::{CLIPSError, CLIPSInto, CLIPSResult};

mod fact_builder;
pub use fact_builder::*;
mod instance_builder;
pub use instance_builder::*;

pub trait FactOrInstanceBuilderData {
    fn put_slot<T: CLIPSInto<CLIPSValue>>(&self, slot_name: &str, val: T) -> CLIPSResult<()>;
    fn put_int_slot<T: Into<i64>>(&self, slot_name: &str, val: T) -> CLIPSResult<()>;
    fn put_float_slot<T: Into<f64>>(&self, slot_name: &str, val: T) -> CLIPSResult<()>;
    fn put_instance_name_slot<T: Into<Vec<u8>>>(&self, slot_name: &str, val: T) -> CLIPSResult<()>;
    fn put_symbol_slot<T: Into<Vec<u8>>>(&self, slot_name: &str, val: T) -> CLIPSResult<()>;
    fn put_string_slot<T: Into<Vec<u8>>>(&self, slot_name: &str, val: T) -> CLIPSResult<()>;
    fn put_multifield_slot<T: CLIPSInto<CLIPSValue>>(
        &self,
        slot_name: &str,
        vals: Vec<T>,
    ) -> CLIPSResult<()>;
}

pub trait IntoFactOrInstance<T: FactOrInstanceBuilderData> {
    fn definition_name(&self) -> &str;
    fn into_fact_or_instance(self: Box<Self>, data: &T) -> CLIPSResult<()>;
}

pub(crate) fn translate_put_slot_error(code: u32) -> CLIPSResult<()> {
    match code {
        clips_sys::PutSlotError_PSE_NO_ERROR => Ok(()),
        clips_sys::PutSlotError_PSE_NULL_POINTER_ERROR => unreachable!(), // We only use the `PutSlot*` functions that take the raw values and create the pointers inside the CLIPS code.
        clips_sys::PutSlotError_PSE_INVALID_TARGET_ERROR => Err(CLIPSError::FactOrInstanceRemoved),
        clips_sys::PutSlotError_PSE_SLOT_NOT_FOUND_ERROR => Err(CLIPSError::SlotNotFound),
        clips_sys::PutSlotError_PSE_TYPE_ERROR => Err(CLIPSError::SlotTypeViolated),
        clips_sys::PutSlotError_PSE_RANGE_ERROR => Err(CLIPSError::SlotRangeViolated),
        clips_sys::PutSlotError_PSE_ALLOWED_VALUES_ERROR => {
            Err(CLIPSError::SlotAllowedValuesViolated)
        }
        clips_sys::PutSlotError_PSE_CARDINALITY_ERROR => Err(CLIPSError::SlotCardinalityViolated),
        clips_sys::PutSlotError_PSE_ALLOWED_CLASSES_ERROR => {
            Err(CLIPSError::SlotAllowedClassesViolated)
        }
        _ => unreachable!(),
    }
}
