// Licensed under the Apache-2.0 license

use crate::cert_store::{cert_slot_mask, SpdmCertStore};
use crate::codec::{Codec, CommonCodec, DataKind, MessageBuf};
use crate::commands::error_rsp::ErrorCode;
use crate::context::SpdmContext;
use crate::error::{CommandError, CommandResult};
use crate::protocol::common::SpdmMsgHdr;
use crate::protocol::{
    AsymAlgo, CertificateInfo, KeyUsageMask, SpdmCertChainHeader, SpdmVersion, SHA384_HASH_SIZE,
    SPDM_CERT_CHAIN_METADATA_LEN, SPDM_MAX_CERT_CHAIN_PORTION_LEN,
};
use crate::state::ConnectionState;
use core::mem::size_of;
use spdmlib_support::hash::{HashAlgoType, HashContext};
use zerocopy::{FromBytes, Immutable, IntoBytes};

#[derive(IntoBytes, FromBytes, Immutable, Default)]
#[repr(C)]
pub struct GetDigestsReq {
    param1: u8,
    param2: u8,
}

impl CommonCodec for GetDigestsReq {
    const DATA_KIND: DataKind = DataKind::Payload;
}

#[derive(IntoBytes, FromBytes, Immutable, Default)]
#[repr(C)]
pub struct GetDigestsRespCommon {
    pub supported_slot_mask: u8,   // param1: introduced in v13
    pub provisioned_slot_mask: u8, // param2
}

impl CommonCodec for GetDigestsRespCommon {
    const DATA_KIND: DataKind = DataKind::Payload;
}

async fn encode_cert_chain_digest<'a>(
    slot_id: u8,
    cert_store: &mut dyn SpdmCertStore,
    asym_algo: AsymAlgo,
    rsp: &mut MessageBuf<'a>,
) -> CommandResult<usize> {
    let crt_chain_len = cert_store
        .cert_chain_len(asym_algo, slot_id)
        .await
        .map_err(|e| (false, CommandError::CertStore(e)))?;
    let cert_chain_format_len = crt_chain_len + SPDM_CERT_CHAIN_METADATA_LEN as usize;

    let header = SpdmCertChainHeader {
        length: cert_chain_format_len as u16,
        reserved: 0,
    };

    // Length and reserved fields
    let header_bytes = header.as_bytes();
    let mut hash_ctx = HashContext::new();
    hash_ctx
        .init(HashAlgoType::SHA384, Some(header_bytes))
        .await
        .map_err(|e| (false, CommandError::Api(e)))?;

    // Root certificate hash
    let mut root_hash = [0u8; SHA384_HASH_SIZE];

    cert_store
        .root_cert_hash(slot_id, asym_algo, &mut root_hash)
        .await
        .map_err(|e| (false, CommandError::CertStore(e)))?;
    hash_ctx
        .update(&root_hash)
        .await
        .map_err(|e| (false, CommandError::Api(e)))?;

    // Hash the certificate chain
    let mut cert_portion = [0u8; SPDM_MAX_CERT_CHAIN_PORTION_LEN as usize];
    let mut offset = 0;

    loop {
        let bytes_read = cert_store
            .get_cert_chain(slot_id, asym_algo, offset, &mut cert_portion)
            .await
            .map_err(|e| (false, CommandError::CertStore(e)))?;

        hash_ctx
            .update(&cert_portion[..bytes_read])
            .await
            .map_err(|e| (false, CommandError::Api(e)))?;

        offset += bytes_read;

        // If the bytes read is less than the length of the cert portion, it indicates the end of the chain
        if bytes_read < cert_portion.len() {
            break;
        }
    }

    // Fill the response buffer with the certificate chain digest
    rsp.put_data(SHA384_HASH_SIZE)
        .map_err(|e| (false, CommandError::Codec(e)))?;
    let cert_chain_digest_buf = rsp
        .data_mut(SHA384_HASH_SIZE)
        .map_err(|_| (false, CommandError::BufferTooSmall))?;
    hash_ctx
        .finalize(cert_chain_digest_buf)
        .await
        .map_err(|e| (false, CommandError::Api(e)))?;
    rsp.pull_data(SHA384_HASH_SIZE)
        .map_err(|_| (false, CommandError::BufferTooSmall))?;

    Ok(SHA384_HASH_SIZE)
}

async fn fill_digests_response<'a>(
    ctx: &mut SpdmContext<'a>,
    connection_version: SpdmVersion,
    rsp: &mut MessageBuf<'a>,
) -> CommandResult<()> {
    // Ensure the selected hash algorithm is SHA384 and retrieve the asymmetric algorithm (currently only ECC-P384 is supported)
    ctx.verify_selected_hash_algo()
        .map_err(|_| ctx.generate_error_response(rsp, ErrorCode::Unspecified, 0, None))?;
    let asym_algo = ctx
        .selected_asym_algo()
        .map_err(|_| ctx.generate_error_response(rsp, ErrorCode::Unspecified, 0, None))?;

    // Get the supported and provisioned slot masks.
    let (supported_slot_mask, provisioned_slot_mask) = cert_slot_mask(ctx.device_certs_store);

    // No slots provisioned with certificates
    let slot_cnt = provisioned_slot_mask.count_ones() as usize;
    if slot_cnt == 0 {
        Err(ctx.generate_error_response(rsp, ErrorCode::Unspecified, 0, None))?;
    }

    // Start filling the response payload
    let mut payload_len = 0;

    // Fill the response header with param1 and param2
    let dgst_rsp_common = GetDigestsRespCommon {
        supported_slot_mask,
        provisioned_slot_mask,
    };

    payload_len += dgst_rsp_common
        .encode(rsp)
        .map_err(|_| (false, CommandError::BufferTooSmall))?;

    // Encode the certificate chain digests for each provisioned slot
    for slot_id in 0..slot_cnt {
        payload_len +=
            encode_cert_chain_digest(slot_id as u8, ctx.device_certs_store, asym_algo, rsp)
                .await
                .map_err(|_| ctx.generate_error_response(rsp, ErrorCode::Unspecified, 0, None))?;
    }

    // Fill the multi-key connection response data if applicable
    if connection_version >= SpdmVersion::V13 && ctx.state.connection_info.multi_key_conn_rsp() {
        payload_len += encode_multi_key_conn_rsp_data(ctx, provisioned_slot_mask, rsp)?;
    }

    // Push data offset up by total payload length
    rsp.push_data(payload_len)
        .map_err(|_| (false, CommandError::BufferTooSmall))?;

    Ok(())
}

fn encode_multi_key_conn_rsp_data(
    ctx: &mut SpdmContext,
    provisioned_slot_mask: u8,
    rsp: &mut MessageBuf,
) -> CommandResult<usize> {
    let slot_cnt = provisioned_slot_mask.count_ones() as usize;

    let key_pair_ids_size = size_of::<u8>() * slot_cnt;
    let cert_infos_size = size_of::<CertificateInfo>() * slot_cnt;
    let key_usage_masks_size = size_of::<KeyUsageMask>() * slot_cnt;
    let total_size = key_pair_ids_size + cert_infos_size + key_usage_masks_size;

    rsp.put_data(total_size)
        .map_err(|_| (false, CommandError::BufferTooSmall))?;
    let data_buf = rsp
        .data_mut(total_size)
        .map_err(|_| (false, CommandError::BufferTooSmall))?;
    data_buf.fill(0);

    let (key_pair_buf, rest) = data_buf.split_at_mut(key_pair_ids_size);
    let (cert_info_buf, key_usage_mask_buf) = rest.split_at_mut(cert_infos_size);

    let mut key_pair_offset = 0;
    let mut key_usage_offset = 0;
    let mut cert_info_offset = 0;

    for slot_id in 0..slot_cnt {
        let key_pair_id = ctx
            .device_certs_store
            .key_pair_id(slot_id as u8)
            .unwrap_or_default();
        let cert_info = ctx
            .device_certs_store
            .cert_info(slot_id as u8)
            .unwrap_or_default();
        let key_usage_mask = ctx
            .device_certs_store
            .key_usage_mask(slot_id as u8)
            .unwrap_or_default();

        // Fill the KeyPairIDs
        key_pair_buf[key_pair_offset..key_pair_offset + size_of::<u8>()]
            .copy_from_slice(key_pair_id.as_bytes());
        key_pair_offset += size_of::<u8>();

        // Fill the CertificateInfos
        cert_info_buf[cert_info_offset..cert_info_offset + size_of::<CertificateInfo>()]
            .copy_from_slice(cert_info.as_bytes());
        cert_info_offset += size_of::<CertificateInfo>();

        // Fill the KeyUsageMasks
        key_usage_mask_buf[key_usage_offset..key_usage_offset + size_of::<KeyUsageMask>()]
            .copy_from_slice(key_usage_mask.as_bytes());
        key_usage_offset += size_of::<KeyUsageMask>();
    }
    rsp.pull_data(total_size)
        .map_err(|_| (false, CommandError::BufferTooSmall))?;

    Ok(total_size)
}

pub(crate) async fn handle_digests<'a>(
    ctx: &mut SpdmContext<'a>,
    spdm_hdr: SpdmMsgHdr,
    req_payload: &mut MessageBuf<'a>,
) -> CommandResult<()> {
    // Validate the connection state
    if ctx.state.connection_info.state() < ConnectionState::AlgorithmsNegotiated {
        Err(ctx.generate_error_response(req_payload, ErrorCode::UnexpectedRequest, 0, None))?;
    }

    // Validate the version
    let connection_version = ctx.state.connection_info.version_number();
    match spdm_hdr.version() {
        Ok(version) if version == connection_version => {}
        _ => Err(ctx.generate_error_response(req_payload, ErrorCode::VersionMismatch, 0, None))?,
    }

    let req = GetDigestsReq::decode(req_payload).map_err(|_| {
        ctx.generate_error_response(req_payload, ErrorCode::InvalidRequest, 0, None)
    })?;

    // Reserved fields must be zero - or unexpected request error
    if req.param1 != 0 || req.param2 != 0 {
        Err(ctx.generate_error_response(req_payload, ErrorCode::UnexpectedRequest, 0, None))?;
    }

    // Check if the certificate capability is supported
    if ctx.local_capabilities.flags.cert_cap() == 0 {
        Err(ctx.generate_error_response(req_payload, ErrorCode::UnsupportedRequest, 0, None))?;
    }

    // TODO: transcript manager and session support

    // Prepare the response buffer
    ctx.prepare_response_buffer(req_payload)?;

    // Fill the response buffer
    fill_digests_response(ctx, connection_version, req_payload).await?;

    // TODO: transcript manager and session support

    if ctx.state.connection_info.state() < ConnectionState::AfterDigest {
        ctx.state
            .connection_info
            .set_state(ConnectionState::AfterDigest);
    }

    Ok(())
}
