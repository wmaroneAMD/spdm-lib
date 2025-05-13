// Licensed under the Apache-2.0 license

use crate::protocol::{DeviceAlgorithms, DeviceCapabilities, SpdmVersion};

pub(crate) struct State {
    pub(crate) connection_info: ConnectionInfo,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            connection_info: ConnectionInfo::default(),
        }
    }

    pub fn reset(&mut self) {
        self.connection_info.reset();
    }
}

pub(crate) struct ConnectionInfo {
    version_number: SpdmVersion,
    state: ConnectionState,
    peer_capabilities: DeviceCapabilities,
    peer_algorithms: DeviceAlgorithms,
    multi_key_conn_rsp: bool,
}

impl Default for ConnectionInfo {
    fn default() -> Self {
        Self {
            version_number: SpdmVersion::default(),
            state: ConnectionState::NotStarted,
            peer_capabilities: DeviceCapabilities::default(),
            peer_algorithms: DeviceAlgorithms::default(),
            multi_key_conn_rsp: false,
        }
    }
}

impl ConnectionInfo {
    pub fn version_number(&self) -> SpdmVersion {
        self.version_number
    }

    pub fn set_version_number(&mut self, version_number: SpdmVersion) {
        self.version_number = version_number;
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn set_state(&mut self, state: ConnectionState) {
        self.state = state;
    }

    pub fn set_peer_capabilities(&mut self, peer_capabilities: DeviceCapabilities) {
        self.peer_capabilities = peer_capabilities;
    }

    #[allow(dead_code)]
    pub fn peer_capabilities(&self) -> DeviceCapabilities {
        self.peer_capabilities
    }

    pub fn set_peer_algorithms(&mut self, peer_algorithms: DeviceAlgorithms) {
        self.peer_algorithms = peer_algorithms;
    }

    pub fn peer_algorithms(&self) -> &DeviceAlgorithms {
        &self.peer_algorithms
    }

    #[allow(dead_code)]
    pub fn set_multi_key_conn_rsp(&mut self, multi_key_conn_rsp: bool) {
        self.multi_key_conn_rsp = multi_key_conn_rsp;
    }

    pub fn multi_key_conn_rsp(&self) -> bool {
        self.multi_key_conn_rsp
    }

    fn reset(&mut self) {
        self.version_number = SpdmVersion::default();
        self.state = ConnectionState::NotStarted;
        self.peer_capabilities = DeviceCapabilities::default();
        self.peer_algorithms = DeviceAlgorithms::default();
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum ConnectionState {
    NotStarted,
    AfterVersion,
    AfterCapabilities,
    AlgorithmsNegotiated,
    AfterDigest,
    AfterCertificate,
}
