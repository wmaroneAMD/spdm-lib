// Licensed under the Apache-2.0 license

// use crate::cert_mgr::DeviceCertsMgrError;
use crate::cert_store::CertStoreError;
use crate::codec::CodecError;
use crate::commands::error_rsp::ErrorCode;

use spdmlib_support::error::*;
use spdmlib_support::hash::*;

#[derive(Debug)]
pub enum SpdmError {
    UnsupportedVersion,
    InvalidParam,
    Codec,
    Transport,
    Command,
    BufferTooSmall,
    UnsupportedRequest,
    CertStore,
}

pub type SpdmResult<T> = Result<T, SpdmError>;

pub type CommandResult<T> = Result<T, (bool, CommandError)>;

#[derive(Debug, PartialEq)]
pub enum CommandError {
    BufferTooSmall,
    Codec(CodecError),
    ErrorCode(ErrorCode),
    UnsupportedRequest,
    CertStore(CertStoreError),
    Api(ApiError),
    Hash(HashError),
}
