// Licensed under the Apache-2.0 license

// use crate::cert_mgr::DeviceCertsMgrError;
use crate::cert_store::CertStoreError;
use crate::codec::CodecError;
use crate::commands::error_rsp::ErrorCode;
use crate::transport::TransportError;

use spdmlib_support::error::*;

#[derive(Debug)]
pub enum SpdmError {
    UnsupportedVersion,
    InvalidParam,
    Codec(CodecError),
    Transport(TransportError),
    Command(CommandError),
    BufferTooSmall,
    UnsupportedRequest,
    CertStore(CertStoreError),
    Error(ApiError),
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
}
