use std::ffi::{c_void, CStr};

use crate::{CLIPSEnvironment, CLIPSSignal, UDFData};

pub type RegisterableRouter = Box<dyn Router + Send + Sync>;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RouterSupport: usize {
        const WRITE = 1 << 0;
        const READ = 1 << 1;
        const SIGNAL = 1 << 2;
    }
}

pub trait Router {
    fn supports(&self) -> RouterSupport;
    fn query(&mut self, logical_name: &str) -> bool;
    fn write(&mut self, _logical_name: &str, _data: &CStr) {}
    fn read(&mut self, _logical_name: &str) -> Option<i32> {
        None
    }
    fn unread(&mut self, _logical_name: &str, _data: i32) -> Option<i32> {
        None
    }
    fn exit(&mut self, _exit_code: i32) {}
    // This is an extension to allow routers to get extra information about the execution of the CLIPS environment, respecting some order. For example, a signal that "Run()" finished will be received by the router only after all the data sent by the CLIPS code was received by the router.
    fn signal(&mut self, _signal: CLIPSSignal) {}
}

pub(crate) extern "C" fn router_query(
    environment: *mut clips_sys::Environment,
    logical_name: *const i8,
    router_name: *mut c_void,
) -> bool {
    let router_name = unsafe { CStr::from_ptr(router_name as *const i8) };
    let router_name_str = router_name.to_str().unwrap();

    let logical_name = unsafe { CStr::from_ptr(logical_name) };
    let logical_name = logical_name.to_str().unwrap();

    let env = CLIPSEnvironment::from_raw(environment);
    let mut router_map = env.retrieve_router_map();
    let router = router_map.get_mut(router_name_str).unwrap();

    let res = router.query(logical_name);
    env.store_router_map(router_map);
    res
}

pub(crate) extern "C" fn router_write(
    environment: *mut clips_sys::Environment,
    logical_name: *const i8,
    data: *const i8,
    router_name: *mut c_void,
) {
    let router_name = unsafe { CStr::from_ptr(router_name as *const i8) };
    let router_name_str = router_name.to_str().unwrap();

    let logical_name = unsafe { CStr::from_ptr(logical_name) };
    let logical_name = logical_name.to_str().unwrap();

    let data = unsafe { CStr::from_ptr(data) };

    let env = CLIPSEnvironment::from_raw(environment);
    let mut router_map = env.retrieve_router_map();
    let router = router_map.get_mut(router_name_str).unwrap();

    let res = router.write(logical_name, data);
    env.store_router_map(router_map);
    res
}

pub(crate) extern "C" fn router_read(
    environment: *mut clips_sys::Environment,
    logical_name: *const i8,
    router_name: *mut c_void,
) -> i32 {
    let router_name = unsafe { CStr::from_ptr(router_name as *const i8) };
    let router_name_str = router_name.to_str().unwrap();

    let logical_name = unsafe { CStr::from_ptr(logical_name) };
    let logical_name = logical_name.to_str().unwrap();

    let env = CLIPSEnvironment::from_raw(environment);
    let mut router_map = env.retrieve_router_map();
    let router = router_map.get_mut(router_name_str).unwrap();

    let res = router.read(logical_name).unwrap_or(-1);
    env.store_router_map(router_map);
    res
}

pub(crate) extern "C" fn router_unread(
    environment: *mut clips_sys::Environment,
    logical_name: *const i8,
    data: i32,
    router_name: *mut c_void,
) -> i32 {
    let router_name = unsafe { CStr::from_ptr(router_name as *const i8) };
    let router_name_str = router_name.to_str().unwrap();

    let logical_name = unsafe { CStr::from_ptr(logical_name) };
    let logical_name = logical_name.to_str().unwrap();

    let env = CLIPSEnvironment::from_raw(environment);
    let mut router_map = env.retrieve_router_map();
    let router = router_map.get_mut(router_name_str).unwrap();

    let res = router.unread(logical_name, data).unwrap_or(-1);
    env.store_router_map(router_map);
    res
}

pub(crate) extern "C" fn router_exit(
    environment: *mut clips_sys::Environment,
    exit_code: i32,
    router_name: *mut c_void,
) {
    let router_name = unsafe { CStr::from_ptr(router_name as *const i8) };
    let router_name_str = router_name.to_str().unwrap();

    let env = CLIPSEnvironment::from_raw(environment);
    let mut router_map = env.retrieve_router_map();
    let router = router_map.get_mut(router_name_str).unwrap();

    router.exit(exit_code);
    env.store_router_map(router_map);
}

pub(crate) extern "C" fn call_udf(
    environment: *mut clips_sys::Environment,
    context: *mut clips_sys::UDFContext,
    udf_result: *mut clips_sys::UDFValue,
) {
    let udf_name = unsafe { CStr::from_ptr(context.as_ref().unwrap().context as *const i8) };
    let udf_name_str = udf_name.to_str().unwrap();

    let env = CLIPSEnvironment::from_raw(environment);
    let mut udf_map = env.retrieve_udf_map();
    let function = udf_map.get_mut(udf_name_str).unwrap();

    let data = UDFData::new(environment, context, udf_result);
    function(data);
    env.store_udf_map(udf_map);
}
