use thiserror::Error;
#[derive(Error, Debug, PartialEq)]
pub enum MacroCircomError {
    #[error("The defined number of expected program utxos as inputs to the circuit is invalid the minimum is 1 the maximum is 4.")]
    InvalidNumberAppUtxos,
    #[error("No instance of {0} found")]
    ParseInstanceError(String),
    #[error("#[lightTransaction] template needs to be defined to compile a psp to circom.")]
    LightTransactionUndefined,
    #[error("Two or more #[instance] objects found, currently only one is supported.")]
    TooManyInstances,
    #[error("No instance defined, an instance object needs to be defined to generate a circom main file")]
    NoInstanceDefined,
    #[error("StringParseError")]
    StringParseError,
    #[error("CheckUtxoInvalidFormat")]
    CheckUtxoInvalidFormat,
    #[error("CheckUtxoInvalidHeaderFormat")]
    CheckUtxoInvalidHeaderFormat,
    #[error("PropertyDefinedMultipleTimes")]
    PropertyDefinedMultipleTimes,
    #[error("InvalidProperty")]
    InvalidProperty,
    #[error("InvalidComparator: {0}")]
    InvalidComparator(String),
    #[error("Duplicate Utxo check: {0}")]
    DuplicateUtxoCheck(String),
    #[error("CheckUtxosNotUsed: {0}")]
    CheckUtxosNotUsed(String),
}
