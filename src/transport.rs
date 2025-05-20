// Licensed under the Apache-2.0 license

extern crate alloc;
use crate::codec::MessageBuf;
use crate::codec::{Codec, CodecError, CommonCodec, DataKind};
use alloc::boxed::Box;
use async_trait::async_trait;
use bitfield::bitfield;
use zerocopy::{FromBytes, Immutable, IntoBytes};

#[derive(Debug)]
pub enum ErrorKind {
    DriverError,
    BufferTooSmall,
    Codec,
    UnexpectedMessageType,
    ReceiveError,
    SendError,
    ResponseNotExpected,
    NoRequestInFlight,
}

pub trait Error: core::fmt::Debug {
    fn kind(&self) -> ErrorKind;
}

pub trait ErrorType {
    type Error: Error;
}

#[async_trait]
pub trait SpdmTransport: ErrorType {
    async fn send_request<'a>(&mut self, dest_eid: u8, req: &mut MessageBuf<'a>) -> Result<(), Self::Error>; 
    async fn send_response<'a>(&mut self, resp: &mut MessageBuf<'a>) -> Result<(), Self::Error>;
    async fn receive_response<'a>(&mut self, rsp: &mut MessageBuf<'a>) -> Result<(), Self::Error>;
    async fn receive_request<'a>(&mut self, req: &mut MessageBuf<'a>) -> Result<(), Self::Error>;
    fn max_message_size(&self) -> usize;
    fn header_size(&self) -> usize;
}
