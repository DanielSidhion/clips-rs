use std::{
    collections::HashMap,
    env::set_current_dir,
    ffi::{CStr, CString},
    mem::size_of,
    path::{Path, PathBuf},
    ptr,
    sync::mpsc,
    thread::{self, JoinHandle},
};

use nix::sched::{unshare, CloneFlags};
use oneshot::SendError;

pub use clips_sys::{CLIPSInstanceName, CLIPSSymbol};

mod router;
pub use router::*;
mod udf;
pub use udf::*;
mod error;
pub use error::*;
mod value;
pub use value::*;
mod fact_instance;
pub use fact_instance::*;

// TODO: find a way to grab these from clips_sys and still be static.
pub static STDOUT: &str = "stdout";
pub static STDERR: &str = "stderr";
pub static STDIN: &str = "stdin";
pub static STDWRN: &str = "stdwrn";

pub type CLIPSGlobalsHierarchy = HashMap<String, HashMap<String, CLIPSValue>>;

#[repr(u32)]
pub enum ConflictResolutionStrategy {
    Depth = clips_sys::StrategyType_DEPTH_STRATEGY,
    Breadth = clips_sys::StrategyType_BREADTH_STRATEGY,
    Lex = clips_sys::StrategyType_LEX_STRATEGY,
    Mea = clips_sys::StrategyType_MEA_STRATEGY,
    Complexity = clips_sys::StrategyType_COMPLEXITY_STRATEGY,
    Simplicity = clips_sys::StrategyType_SIMPLICITY_STRATEGY,
    Random = clips_sys::StrategyType_RANDOM_STRATEGY,
}

pub trait CLIPSFrom<T> {
    fn from(value: T, env: *mut clips_sys::Environment) -> Self;
}

pub trait CLIPSInto<T> {
    fn into(self, env: *mut clips_sys::Environment) -> T;
}

impl<T, U> CLIPSInto<U> for T
where
    U: CLIPSFrom<T>,
{
    fn into(self, env: *mut clips_sys::Environment) -> U {
        U::from(self, env)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CLIPSSignal {
    RunStarted { limit: Option<usize> },
    RunFinished { limit: Option<usize> },
}

#[derive(Debug)]
pub struct Environment {
    input_tx: mpsc::Sender<CLIPSEnvironmentCommand>,
    task_handle: JoinHandle<()>,
}

impl Environment {
    pub fn new() -> Self {
        let (input_tx, input_rx) = mpsc::channel();

        let task_handle = thread::spawn(move || clips_environment_task(input_rx));

        Self {
            input_tx,
            task_handle,
        }
    }

    pub fn close(self) -> CLIPSResult<()> {
        self.input_tx
            .send(CLIPSEnvironmentCommand::Close)
            .map_err(|_| CLIPSError::ThreadExited)?;
        self.task_handle
            .join()
            .map_err(|_| CLIPSError::TaskExitedUnexpectedly)?;
        Ok(())
    }

    pub fn load_from_str(&self, data: &str) -> CLIPSResult<()> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::LoadFromStr {
                data: data.to_string(),
                res_tx,
            })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn batch_star(&self, file_path: PathBuf) -> CLIPSResult<()> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::BatchStar { file_path, res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn chdir(&self, new_dir: PathBuf) -> CLIPSResult<()> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::ChDir { new_dir, res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn run(&self) -> CLIPSResult<usize> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::Run { res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn add_udf(
        &self,
        name: String,
        min_args: u16,
        max_args: u16,
        return_types: UDFType,
        arg_types: Vec<UDFType>,
        function: Box<dyn FnMut(UDFData) + Send + Sync>,
    ) -> CLIPSResult<()> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::AddUDF {
                name,
                min_args,
                max_args,
                return_types,
                arg_types,
                function,
                res_tx,
            })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn add_router(
        &self,
        name: String,
        priority: i32,
        router: RegisterableRouter,
    ) -> CLIPSResult<()> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::AddRouter {
                name,
                priority,
                router,
                res_tx,
            })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn remove_udf(&self, name: String) -> CLIPSResult<bool> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::RemoveUDF { name, res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)
    }

    pub fn assert_fact<T: IntoFactOrInstance<FactBuilderData> + Send + Sync + 'static>(
        &self,
        value: T,
    ) -> CLIPSResult<()> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::AssertFact {
                value: Box::new(value),
                res_tx,
            })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn make_instance<T: IntoFactOrInstance<InstanceBuilderData> + Send + Sync + 'static>(
        &self,
        value: T,
        instance_name: Option<String>,
    ) -> CLIPSResult<()> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::MakeInstance {
                value: Box::new(value),
                instance_name,
                res_tx,
            })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn set_dynamic_constraint_checking(&self, value: bool) -> CLIPSResult<()> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::SetDynamicConstraintChecking { value, res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        Ok(res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?)
    }

    pub fn set_conflict_resolution_strategy(
        &self,
        value: ConflictResolutionStrategy,
    ) -> CLIPSResult<()> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::SetConflictResolutionStrategy { value, res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        Ok(res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?)
    }

    pub fn get_current_parsing_location(&self) -> CLIPSResult<(String, usize)> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::GetCurrentParsingLocation { res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        Ok(res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?)
    }

    pub fn binary_save_facts(&self, path: PathBuf) -> CLIPSResult<usize> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::BinarySaveFacts { path, res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn binary_load_facts(&self, path: PathBuf) -> CLIPSResult<usize> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::BinaryLoadFacts { path, res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn binary_save_instances(&self, path: PathBuf) -> CLIPSResult<usize> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::BinarySaveInstances { path, res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn binary_load_instances(&self, path: PathBuf) -> CLIPSResult<usize> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::BinaryLoadInstances { path, res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn retrieve_globals_values(&self) -> CLIPSResult<CLIPSGlobalsHierarchy> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::RetrieveGlobalsValues { res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }

    pub fn restore_globals(&self, globals: CLIPSGlobalsHierarchy) -> CLIPSResult<()> {
        let (res_tx, res_rx) = oneshot::channel();

        self.input_tx
            .send(CLIPSEnvironmentCommand::RestoreGlobals { globals, res_tx })
            .map_err(|_| CLIPSError::ThreadExited)?;

        res_rx.recv().map_err(|_| CLIPSError::ThreadExited)?
    }
}

enum CLIPSEnvironmentCommand {
    LoadFromStr {
        data: String,
        res_tx: oneshot::Sender<CLIPSResult<()>>,
    },
    BatchStar {
        file_path: PathBuf,
        res_tx: oneshot::Sender<CLIPSResult<()>>,
    },
    Run {
        res_tx: oneshot::Sender<CLIPSResult<usize>>,
    },
    RunLimit {
        limit: usize,
        res_tx: oneshot::Sender<CLIPSResult<usize>>,
    },
    ChDir {
        new_dir: PathBuf,
        res_tx: oneshot::Sender<CLIPSResult<()>>,
    },
    AddUDF {
        name: String,
        min_args: u16,
        max_args: u16,
        return_types: UDFType,
        arg_types: Vec<UDFType>,
        function: Box<dyn FnMut(UDFData) + Send + Sync>,
        res_tx: oneshot::Sender<CLIPSResult<()>>,
    },
    AddRouter {
        name: String,
        priority: i32,
        router: RegisterableRouter,
        res_tx: oneshot::Sender<CLIPSResult<()>>,
    },
    RemoveUDF {
        name: String,
        res_tx: oneshot::Sender<bool>,
    },
    AssertFact {
        value: Box<dyn IntoFactOrInstance<FactBuilderData> + Send + Sync>,
        res_tx: oneshot::Sender<CLIPSResult<()>>,
    },
    MakeInstance {
        value: Box<dyn IntoFactOrInstance<InstanceBuilderData> + Send + Sync>,
        instance_name: Option<String>,
        res_tx: oneshot::Sender<CLIPSResult<()>>,
    },
    SetDynamicConstraintChecking {
        value: bool,
        res_tx: oneshot::Sender<()>,
    },
    SetConflictResolutionStrategy {
        value: ConflictResolutionStrategy,
        res_tx: oneshot::Sender<()>,
    },
    GetCurrentParsingLocation {
        res_tx: oneshot::Sender<(String, usize)>,
    },
    BinarySaveFacts {
        path: PathBuf,
        res_tx: oneshot::Sender<CLIPSResult<usize>>,
    },
    BinaryLoadFacts {
        path: PathBuf,
        res_tx: oneshot::Sender<CLIPSResult<usize>>,
    },
    BinarySaveInstances {
        path: PathBuf,
        res_tx: oneshot::Sender<CLIPSResult<usize>>,
    },
    BinaryLoadInstances {
        path: PathBuf,
        res_tx: oneshot::Sender<CLIPSResult<usize>>,
    },
    RetrieveGlobalsValues {
        res_tx: oneshot::Sender<CLIPSResult<CLIPSGlobalsHierarchy>>,
    },
    RestoreGlobals {
        globals: CLIPSGlobalsHierarchy,
        res_tx: oneshot::Sender<CLIPSResult<()>>,
    },
    Close,
}

fn clips_environment_task(input_rx: mpsc::Receiver<CLIPSEnvironmentCommand>) {
    // We use `unshare()` to allow this thread setting a different `chdir` than other threads in the process. This library expects to be used in multi-threaded programs, and by default `chdir()` applies to the entire process.
    unshare(CloneFlags::CLONE_FS).unwrap();

    let mut env = CLIPSEnvironment::new().unwrap();

    // In the loop below, we'll ignore any `SendError`s that happen when sending the result of doing the work that was requested. To do this with some concise code, we must get rid of the `SendError`s  returned by each channel's `send()` call, because those errors all have different types (and thus can't be assigned to the same variable). The `StubError` below exists so we can map all `SendError`s to a `StubError` to allow the code to be concise.
    struct StubError {}
    fn create_stub_error<T>(_prev: SendError<T>) -> StubError {
        StubError {}
    }

    loop {
        let result_res = match input_rx.recv() {
            Err(_) => {
                log::info!("The input channel for the CLIPS environment is closed, so will stop the CLIPS environment task.");
                break;
            }
            Ok(CLIPSEnvironmentCommand::Close) => {
                log::info!("Got asked to close the CLIPS environment. Stopping the CLIPS environment task.");
                break;
            }
            Ok(CLIPSEnvironmentCommand::LoadFromStr { data, res_tx }) => res_tx
                .send(env.load_from_str(&data))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::Run { res_tx }) => {
                res_tx.send(env.run()).map_err(create_stub_error)
            }
            Ok(CLIPSEnvironmentCommand::RunLimit { limit, res_tx }) => {
                res_tx.send(env.run_limit(limit)).map_err(create_stub_error)
            }
            Ok(CLIPSEnvironmentCommand::ChDir { new_dir, res_tx }) => res_tx
                .send(set_current_dir(new_dir).map_err(CLIPSError::from))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::BatchStar { file_path, res_tx }) => res_tx
                .send(env.batch_star(file_path))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::AddUDF {
                name,
                min_args,
                max_args,
                return_types,
                arg_types,
                function,
                res_tx,
            }) => res_tx
                .send(env.add_udf(&name, return_types, min_args, max_args, arg_types, function))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::AddRouter {
                name,
                priority,
                router,
                res_tx,
            }) => res_tx
                .send(env.add_router(&name, priority, router))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::RemoveUDF { name, res_tx }) => res_tx
                .send(env.remove_udf(&name))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::AssertFact { value, res_tx }) => res_tx
                .send(env.assert_fact(value))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::MakeInstance {
                value,
                instance_name,
                res_tx,
            }) => res_tx
                .send(env.make_instance(value, instance_name.as_deref()))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::SetDynamicConstraintChecking { value, res_tx }) => res_tx
                .send(env.set_dynamic_constraint_checking(value))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::SetConflictResolutionStrategy { value, res_tx }) => res_tx
                .send(env.set_conflict_resolution_strategy(value))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::GetCurrentParsingLocation { res_tx }) => res_tx
                .send(env.get_current_parsing_location())
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::BinarySaveFacts { path, res_tx }) => res_tx
                .send(env.binary_save_facts(path))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::BinaryLoadFacts { path, res_tx }) => res_tx
                .send(env.binary_load_facts(path))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::BinarySaveInstances { path, res_tx }) => res_tx
                .send(env.binary_save_instances(path))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::BinaryLoadInstances { path, res_tx }) => res_tx
                .send(env.binary_load_instances(path))
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::RetrieveGlobalsValues { res_tx }) => res_tx
                .send(env.retrieve_globals_values())
                .map_err(create_stub_error),
            Ok(CLIPSEnvironmentCommand::RestoreGlobals { globals, res_tx }) => res_tx
                .send(env.restore_globals(globals))
                .map_err(create_stub_error),
        };

        if let Err(_) = result_res {
            break;
        }
    }
}

const UDF_MAP_ENVIRONMENT_DATA_INDEX: u32 = clips_sys::USER_ENVIRONMENT_DATA + 0;
const ROUTER_MAP_ENVIRONMENT_DATA_INDEX: u32 = clips_sys::USER_ENVIRONMENT_DATA + 1;
const STRINGS_TO_DROP_ENVIRONMENT_DATA_INDEX: u32 = clips_sys::USER_ENVIRONMENT_DATA + 2;

type CLIPSEnvironmentUDFMap = HashMap<String, Box<dyn FnMut(UDFData) + Sync + Send>>;
type CLIPSEnvironmentRouterMap = HashMap<String, RegisterableRouter>;
type CLIPSEnvironmentStringsToDrop = Vec<*const i8>;

pub struct CLIPSEnvironment {
    raw: *mut clips_sys::Environment,
    destroy_on_drop: bool,
    fact_builders: HashMap<String, CLIPSFactBuilder>,
    instance_builders: HashMap<String, CLIPSInstanceBuilder>,
}

impl CLIPSEnvironment {
    pub fn new() -> CLIPSResult<Self> {
        let raw = unsafe { clips_sys::CreateEnvironment() };

        let udf_map: Box<CLIPSEnvironmentUDFMap> = Box::new(HashMap::new());
        let router_map: Box<CLIPSEnvironmentRouterMap> = Box::new(HashMap::new());
        // We unwrap some strings to give them to CLIPS so it can hold onto them while it runs. We also keep a copy of them here, so when we drop the environment we can take back ownership over those strings to properly drop them.
        let strings_to_drop: Box<CLIPSEnvironmentStringsToDrop> = Box::new(Vec::new());

        unsafe {
            let res = clips_sys::AllocateEnvironmentData(
                raw,
                UDF_MAP_ENVIRONMENT_DATA_INDEX,
                size_of::<Box<CLIPSEnvironmentUDFMap>>(),
                Some(cleanup_udf_map),
            );

            if !res {
                return Err(CLIPSError::EnvironmentNotCreated);
            }

            let res = clips_sys::AllocateEnvironmentData(
                raw,
                ROUTER_MAP_ENVIRONMENT_DATA_INDEX,
                size_of::<Box<CLIPSEnvironmentRouterMap>>(),
                Some(cleanup_router_map),
            );

            if !res {
                return Err(CLIPSError::EnvironmentNotCreated);
            }

            let res = clips_sys::AllocateEnvironmentData(
                raw,
                STRINGS_TO_DROP_ENVIRONMENT_DATA_INDEX,
                size_of::<Box<CLIPSEnvironmentStringsToDrop>>(),
                Some(cleanup_strings_to_drop),
            );

            if !res {
                return Err(CLIPSError::EnvironmentNotCreated);
            }

            clips_sys::SetEnvironmentData(
                raw,
                UDF_MAP_ENVIRONMENT_DATA_INDEX,
                Box::into_raw(udf_map) as *mut _,
            );
            clips_sys::SetEnvironmentData(
                raw,
                ROUTER_MAP_ENVIRONMENT_DATA_INDEX,
                Box::into_raw(router_map) as *mut _,
            );
            clips_sys::SetEnvironmentData(
                raw,
                STRINGS_TO_DROP_ENVIRONMENT_DATA_INDEX,
                Box::into_raw(strings_to_drop) as *mut _,
            );
        }

        Ok(Self {
            raw,
            destroy_on_drop: true,
            fact_builders: HashMap::new(),
            instance_builders: HashMap::new(),
        })
    }

    pub fn from_raw(raw: *mut clips_sys::Environment) -> Self {
        Self {
            raw,
            destroy_on_drop: false,
            fact_builders: HashMap::new(),
            instance_builders: HashMap::new(),
        }
    }

    pub(crate) fn retrieve_udf_map(&self) -> Box<CLIPSEnvironmentUDFMap> {
        unsafe {
            let udf_map_ptr =
                clips_sys::GetEnvironmentData(self.raw, UDF_MAP_ENVIRONMENT_DATA_INDEX)
                    as *mut CLIPSEnvironmentUDFMap;

            Box::from_raw(udf_map_ptr)
        }
    }

    pub(crate) fn store_udf_map(&self, map: Box<CLIPSEnvironmentUDFMap>) {
        unsafe {
            clips_sys::SetEnvironmentData(
                self.raw,
                UDF_MAP_ENVIRONMENT_DATA_INDEX,
                Box::into_raw(map) as *mut _,
            );
        }
    }

    pub(crate) fn retrieve_router_map(&self) -> Box<CLIPSEnvironmentRouterMap> {
        unsafe {
            let router_map_ptr =
                clips_sys::GetEnvironmentData(self.raw, ROUTER_MAP_ENVIRONMENT_DATA_INDEX)
                    as *mut CLIPSEnvironmentRouterMap;

            Box::from_raw(router_map_ptr)
        }
    }

    pub(crate) fn store_router_map(&self, map: Box<CLIPSEnvironmentRouterMap>) {
        unsafe {
            clips_sys::SetEnvironmentData(
                self.raw,
                ROUTER_MAP_ENVIRONMENT_DATA_INDEX,
                Box::into_raw(map) as *mut _,
            );
        }
    }

    pub(crate) fn retrieve_strings_to_drop(&self) -> Box<CLIPSEnvironmentStringsToDrop> {
        unsafe {
            let strings_to_drop_ptr =
                clips_sys::GetEnvironmentData(self.raw, STRINGS_TO_DROP_ENVIRONMENT_DATA_INDEX)
                    as *mut CLIPSEnvironmentStringsToDrop;

            Box::from_raw(strings_to_drop_ptr)
        }
    }

    pub(crate) fn store_strings_to_drop(&self, map: Box<CLIPSEnvironmentStringsToDrop>) {
        unsafe {
            clips_sys::SetEnvironmentData(
                self.raw,
                STRINGS_TO_DROP_ENVIRONMENT_DATA_INDEX,
                Box::into_raw(map) as *mut _,
            );
        }
    }

    fn send_routers_signal(&mut self, signal: CLIPSSignal) {
        // TODO: optimise this by storing a list of routers that have SIGNAL support without having to check every time?
        let mut router_map = self.retrieve_router_map();
        for router in router_map.values_mut() {
            if router.supports().contains(RouterSupport::SIGNAL) {
                router.signal(signal.clone());
            }
        }
        self.store_router_map(router_map);
    }

    pub fn load_from_str(&mut self, data: &str) -> CLIPSResult<()> {
        let res =
            unsafe { clips_sys::LoadFromString(self.raw, data.as_ptr() as *const i8, data.len()) };

        if !res {
            Err(CLIPSError::LoadFromString)
        } else {
            Ok(())
        }
    }

    pub fn batch_star<P: AsRef<Path>>(&mut self, file_path: P) -> CLIPSResult<()> {
        let path_str = file_path
            .as_ref()
            .to_str()
            .ok_or_else(|| CLIPSError::PathNotUnicode)?;

        let path_cstring = CString::new(path_str).unwrap();
        let res = unsafe { clips_sys::BatchStar(self.raw, path_cstring.as_ptr()) };

        if !res {
            Err(CLIPSError::BatchStar)
        } else {
            Ok(())
        }
    }

    pub fn run(&mut self) -> CLIPSResult<usize> {
        self.send_routers_signal(CLIPSSignal::RunStarted { limit: None });
        let rules_ran = unsafe { clips_sys::Run(self.raw, -1) };
        self.send_routers_signal(CLIPSSignal::RunFinished { limit: None });

        Ok(rules_ran as usize)
    }

    pub fn run_limit(&mut self, limit: usize) -> CLIPSResult<usize> {
        self.send_routers_signal(CLIPSSignal::RunStarted { limit: Some(limit) });
        let rules_ran = unsafe { clips_sys::Run(self.raw, limit as i64) };
        self.send_routers_signal(CLIPSSignal::RunFinished { limit: Some(limit) });

        Ok(rules_ran as usize)
    }

    pub fn add_udf(
        &mut self,
        name: &str,
        return_types: UDFType,
        min_args: u16,
        max_args: u16,
        arg_types: Vec<UDFType>,
        function: Box<dyn FnMut(UDFData) + Send + Sync>,
    ) -> CLIPSResult<()> {
        let arg_types: String = arg_types
            .into_iter()
            .map(|a| a.as_character_code())
            .collect::<Vec<_>>()
            .join(";");
        let arg_types = CString::new(arg_types).unwrap();
        let return_types = CString::new(return_types.as_character_code()).unwrap();

        let mut udf_map = self.retrieve_udf_map();
        udf_map.insert(name.to_string(), function);
        self.store_udf_map(udf_map);

        let name_str = CString::new(name).unwrap().into_raw();
        let mut strings_to_drop = self.retrieve_strings_to_drop();
        strings_to_drop.push(name_str);
        self.store_strings_to_drop(strings_to_drop);

        let res = unsafe {
            clips_sys::AddUDF(
                self.raw,
                name_str as *const i8,
                return_types.as_ptr(),
                min_args,
                max_args,
                arg_types.as_ptr(),
                Some(call_udf),
                name_str as *const i8,
                name_str as *mut _,
            )
        };

        match res {
            clips_sys::AddUDFError_AUE_NO_ERROR => Ok(()),
            clips_sys::AddUDFError_AUE_MIN_EXCEEDS_MAX_ERROR => Err(CLIPSError::MinArgumentsExceedsMax),
            clips_sys::AddUDFError_AUE_FUNCTION_NAME_IN_USE_ERROR => Err(CLIPSError::NameInUse),
            clips_sys::AddUDFError_AUE_INVALID_ARGUMENT_TYPE_ERROR => unreachable!("the library should've generated valid argument types"),
            clips_sys::AddUDFError_AUE_INVALID_RETURN_TYPE_ERROR => unreachable!("the library should've generated valid return types"),
            _ => unreachable!("a new error value for AddUDF was used by CLIPS, but this library doesn't handle it yet"),
        }
    }

    pub fn remove_udf(&mut self, name: &str) -> bool {
        let mut udf_map = self.retrieve_udf_map();
        udf_map.remove(name);
        self.store_udf_map(udf_map);

        let c_str = CString::new(name).unwrap();
        let res = unsafe { clips_sys::RemoveUDF(self.raw, c_str.as_ptr()) };
        res
    }

    pub fn add_router(
        &mut self,
        name: &str,
        priority: i32,
        router: RegisterableRouter,
    ) -> CLIPSResult<()> {
        let supports = router.supports();

        let mut router_map = self.retrieve_router_map();
        router_map.insert(name.to_string(), router);
        self.store_router_map(router_map);

        let name_str = CString::new(name).unwrap().into_raw();
        let mut strings_to_drop = self.retrieve_strings_to_drop();
        strings_to_drop.push(name_str);
        self.store_strings_to_drop(strings_to_drop);

        let res = unsafe {
            clips_sys::AddRouter(
                self.raw,
                name_str as *const i8,
                priority,
                Some(router_query),
                if supports.contains(RouterSupport::WRITE) {
                    Some(router_write)
                } else {
                    None
                },
                if supports.contains(RouterSupport::READ) {
                    Some(router_read)
                } else {
                    None
                },
                if supports.contains(RouterSupport::READ) {
                    Some(router_unread)
                } else {
                    None
                },
                Some(router_exit),
                name_str as *mut _,
            )
        };

        if res {
            Ok(())
        } else {
            Err(CLIPSError::AddRouter)
        }
    }

    pub fn assert_fact(
        &mut self,
        data: Box<dyn IntoFactOrInstance<FactBuilderData>>,
    ) -> CLIPSResult<()> {
        let template_name = data.definition_name();

        let fb = if let Some(fb) = self.fact_builders.get(template_name) {
            fb.fb
        } else {
            let template_name_cstr = CString::new(template_name).unwrap();
            let fb = unsafe { clips_sys::CreateFactBuilder(self.raw, template_name_cstr.as_ptr()) };
            self.fact_builders
                .insert(template_name.to_string(), CLIPSFactBuilder { fb });
            fb
        };

        let fb_data = FactBuilderData::new(fb, self.raw);

        data.into_fact_or_instance(&fb_data)?;
        fb_data.assert()
    }

    pub fn make_instance(
        &mut self,
        data: Box<dyn IntoFactOrInstance<InstanceBuilderData>>,
        instance_name: Option<&str>,
    ) -> CLIPSResult<()> {
        let template_name = data.definition_name();

        let ib = if let Some(ib) = self.instance_builders.get(template_name) {
            ib.ib
        } else {
            let template_name_cstr = CString::new(template_name).unwrap();
            let ib =
                unsafe { clips_sys::CreateInstanceBuilder(self.raw, template_name_cstr.as_ptr()) };
            self.instance_builders
                .insert(template_name.to_string(), CLIPSInstanceBuilder { ib });
            ib
        };

        let ib_data = InstanceBuilderData::new(ib, self.raw);

        data.into_fact_or_instance(&ib_data)?;
        ib_data.make(instance_name)
    }

    pub fn set_dynamic_constraint_checking(&mut self, value: bool) {
        unsafe { clips_sys::SetDynamicConstraintChecking(self.raw, value) };
    }

    pub fn set_conflict_resolution_strategy(&mut self, strategy: ConflictResolutionStrategy) {
        unsafe { clips_sys::SetStrategy(self.raw, strategy as u32) };
    }

    pub fn get_current_parsing_location(&mut self) -> (String, usize) {
        let file_name_ptr = unsafe { clips_sys::GetParsingFileName(self.raw) };
        let file_name = unsafe { CStr::from_ptr(file_name_ptr) };

        let line_number = unsafe { clips_sys::GetLineCount(self.raw) };

        (
            file_name.to_str().unwrap().to_string(),
            line_number as usize,
        )
    }

    pub fn binary_save_facts(&self, path: PathBuf) -> CLIPSResult<usize> {
        let res = unsafe {
            let path_cstr = CString::new(path.into_os_string().as_encoded_bytes()).unwrap();

            clips_sys::BinarySaveFacts(
                self.raw,
                path_cstr.as_ptr(),
                clips_sys::SaveScope_VISIBLE_SAVE,
            )
        };

        if res == -1 {
            Err(CLIPSError::UnableToSaveFacts)
        } else {
            Ok(res as usize)
        }
    }

    pub fn binary_load_facts(&self, path: PathBuf) -> CLIPSResult<usize> {
        let res = unsafe {
            let path_cstr = CString::new(path.into_os_string().as_encoded_bytes()).unwrap();

            clips_sys::BinaryLoadFacts(self.raw, path_cstr.as_ptr())
        };

        if res == -1 {
            Err(CLIPSError::UnableToSaveFacts)
        } else {
            Ok(res as usize)
        }
    }

    pub fn binary_save_instances(&self, path: PathBuf) -> CLIPSResult<usize> {
        let res = unsafe {
            let path_cstr = CString::new(path.into_os_string().as_encoded_bytes()).unwrap();

            clips_sys::BinarySaveInstances(
                self.raw,
                path_cstr.as_ptr(),
                clips_sys::SaveScope_VISIBLE_SAVE,
            )
        };

        if res == -1 {
            Err(CLIPSError::UnableToSaveInstances)
        } else {
            Ok(res as usize)
        }
    }

    pub fn binary_load_instances(&self, path: PathBuf) -> CLIPSResult<usize> {
        let res = unsafe {
            let path_cstr = CString::new(path.into_os_string().as_encoded_bytes()).unwrap();

            clips_sys::BinaryLoadInstances(self.raw, path_cstr.as_ptr())
        };

        if res == -1 {
            Err(CLIPSError::UnableToSaveInstances)
        } else {
            Ok(res as usize)
        }
    }

    // Note: this is an implementation based on the C code for `ShowDefglobals()` (in the CLIPS source code). `ShowDefglobals()` prints to a router, but to avoid the indirection we'll directly iterate through every defglobal (if we decided to call `ShowDefglobals()`, we'd have to define a new router that would parse the printed data, so doing things directly saves us a lot of work).
    pub fn retrieve_globals_values(&self) -> CLIPSResult<CLIPSGlobalsHierarchy> {
        let mut defglobals_hierarchy = HashMap::new();

        let mut defmodule = unsafe { clips_sys::GetNextDefmodule(self.raw, ptr::null_mut()) };
        while !defmodule.is_null() {
            let module_name = unsafe { CStr::from_ptr(clips_sys::DefmoduleName(defmodule)) };
            let module_name_str = module_name.to_str().unwrap();

            if !defglobals_hierarchy.contains_key(module_name_str) {
                defglobals_hierarchy.insert(module_name_str.to_string(), HashMap::new());
            }

            let mut curr_defglobal = unsafe {
                (*clips_sys::GetDefglobalModuleItem(self.raw, defmodule))
                    .header
                    .firstItem as *mut clips_sys::defglobal
            };

            while !curr_defglobal.is_null() {
                let construct_type = unsafe { (*curr_defglobal).header.constructType };
                if construct_type != clips_sys::ConstructType_DEFGLOBAL {
                    return Err(CLIPSError::UnexpectedConstructType(construct_type));
                } else {
                    let name = unsafe { CStr::from_ptr((*(*curr_defglobal).header.name).contents) };
                    let name_str = name.to_str().unwrap();
                    let value = unsafe { (*curr_defglobal).current };

                    defglobals_hierarchy
                        .get_mut(module_name_str)
                        .unwrap()
                        .insert(name_str.to_string(), extract_clipsvalue(value));
                }

                curr_defglobal =
                    unsafe { (*curr_defglobal).header.next as *mut clips_sys::defglobal };
            }

            defmodule = unsafe { clips_sys::GetNextDefmodule(self.raw, defmodule) };
        }

        Ok(defglobals_hierarchy)
    }

    pub fn restore_globals(&self, globals: CLIPSGlobalsHierarchy) -> CLIPSResult<()> {
        for (module_name, globals) in globals {
            for (global_name, global_value) in globals {
                let full_global_name = format!("{}::{}", module_name, global_name);
                let mut raw_value: clips_sys::CLIPSValue = CLIPSInto::into(global_value, self.raw);

                unsafe {
                    let full_name_cstring = CString::new(full_global_name).unwrap();
                    let curr_defglobal =
                        clips_sys::FindDefglobal(self.raw, full_name_cstring.as_ptr());

                    if curr_defglobal.is_null() {
                        return Err(CLIPSError::DefglobalNotFound);
                    }

                    clips_sys::DefglobalSetValue(curr_defglobal, &mut raw_value);
                };
            }
        }

        Ok(())
    }
}

impl Drop for CLIPSEnvironment {
    fn drop(&mut self) {
        if !self.destroy_on_drop {
            return;
        }

        for ib in self.instance_builders.values() {
            unsafe { clips_sys::IBDispose(ib.ib) };
        }

        for fb in self.fact_builders.values() {
            unsafe { clips_sys::FBDispose(fb.fb) };
        }

        let res = unsafe { clips_sys::DestroyEnvironment(self.raw) };

        if !res {
            log::error!("Attempt at destroying CLIPS environment failed!");
        }
    }
}

extern "C" fn cleanup_udf_map(environment: *mut clips_sys::Environment) {
    let env = CLIPSEnvironment::from_raw(environment);
    drop(env.retrieve_udf_map());
}

extern "C" fn cleanup_router_map(environment: *mut clips_sys::Environment) {
    let env = CLIPSEnvironment::from_raw(environment);
    drop(env.retrieve_router_map());
}

extern "C" fn cleanup_strings_to_drop(environment: *mut clips_sys::Environment) {
    let env = CLIPSEnvironment::from_raw(environment);
    let mut strings_to_drop = env.retrieve_strings_to_drop();

    for ptr in strings_to_drop.drain(..) {
        drop(unsafe { CString::from_raw(ptr as *mut i8) });
    }
}
