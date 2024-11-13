use thiserror::Error;

#[derive(Error, Debug)]
pub enum CLIPSError {
    #[error("the CLIPS environment couldn't be successfully created")]
    EnvironmentNotCreated,
    #[error("the given path isn't valid unicode")]
    PathNotUnicode,
    #[error("CLIPS failed to parse the given expression")]
    ParsingError,
    #[error("CLIPS failed to execute the given expression")]
    ProcessingError,
    #[error("CLIPS was unable to load from the given string")]
    LoadFromString,
    #[error("CLIPS was unable to load the given file path")]
    BatchStar,
    #[error("the minimum number of arguments given for this UDF exceeds the given maximum number of arguments")]
    MinArgumentsExceedsMax,
    #[error("the argument couldn't be retrieved because it's either out of bounds or not of the expected type")]
    ArgumentNotRetrieved,
    #[error("this name is already in use")]
    NameInUse,
    #[error("CLIPS failed to add the requested router")]
    AddRouter,
    #[error("CLIPS was unable to change to the requested directory")]
    ChDir,
    #[error("the CLIPS thread exited unexpectedly")]
    ThreadExited,
    #[error("the CLIPS environment task exited unexpectedly")]
    TaskExitedUnexpectedly,
    #[error("an IO error happened")]
    IO(#[from] std::io::Error),
    #[error("failed to convert UDF value: {0}")]
    UDFDataConversion(#[from] clips_sys::UDFConversionError),
    #[error("the fact could not be asserted in the CLIPS environment (possibly pattern matching of a fact or instance is already occurring)")]
    UnableToAssertFact,
    #[error("the instance could not be created in the CLIPS environment (possibly pattern matching of a fact or instance is already occurring)")]
    UnableToMakeInstance,
    #[error("an error occurred while the assertion was being processed in the rule network")]
    RuleNetwork,
    #[error("the fact or instance being modified was removed")]
    FactOrInstanceRemoved,
    #[error("no slot with the given name was found for the selected template")]
    SlotNotFound,
    #[error("the value given violates the type constraint for the slot")]
    SlotTypeViolated,
    #[error("the value given violates the range constraint for the slot")]
    SlotRangeViolated,
    #[error("the value given violates the allowed values constraint for the slot")]
    SlotAllowedValuesViolated,
    #[error("the value given violates the cardinality constraint for the slot")]
    SlotCardinalityViolated,
    #[error("the value given violates the allowed classes constraint for the slot")]
    SlotAllowedClassesViolated,
    #[error("CLIPS encountered an error when trying to save facts to the filename")]
    UnableToSaveFacts,
    #[error("CLIPS encountered an error when trying to load facts from the filename")]
    UnableToLoadFacts,
    #[error("CLIPS encountered an error when trying to save instances to the filename")]
    UnableToSaveInstances,
    #[error("CLIPS encountered an error when trying to load instances from the filename")]
    UnableToLoadInstances,
    #[error("the construct type we got isn't what we expected. Got '{0}'")]
    UnexpectedConstructType(u32),
    #[error("tried to find a defglobal, but it didn't exist")]
    DefglobalNotFound,
    #[error("unknown CLIPS error")]
    Unknown,
}

pub type CLIPSResult<T> = Result<T, CLIPSError>;
