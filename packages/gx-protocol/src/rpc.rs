//! HTTP RPC envelopes (`POST /api/<name>`).

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::GXSERVER_PROTOCOL_VERSION;

/// Request body. `protocolVersion` must be a JSON number: a string `"1"` in the body is rejected
/// with `protocolMismatch`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcRequest<P> {
    pub protocol_version: u64,
    pub params: P,
}

impl<P> RpcRequest<P> {
    pub fn new(params: P) -> Self {
        Self {
            protocol_version: GXSERVER_PROTOCOL_VERSION,
            params,
        }
    }
}

open_string_enum! {
    /// RPC error codes. The set is open: several real codes (`invalidParams`, `composerNotReady`)
    /// arrive with HTTP 500, so a client branches on the code, never on the status.
    RpcErrorCode {
        BadRequest => "badRequest",
        Unauthorized => "unauthorized",
        Forbidden => "forbidden",
        NotFound => "notFound",
        MethodNotAllowed => "methodNotAllowed",
        CorruptState => "corruptState",
        ProjectPathUnavailable => "projectPathUnavailable",
        ProtocolMismatch => "protocolMismatch",
        DependencyUnavailable => "dependencyUnavailable",
        InternalError => "internalError",
        ComposerNotReady => "composerNotReady",
        ComposerNotCleared => "composerNotCleared",
        InvalidParams => "invalidParams",
    }
}

/// Success envelope (HTTP 200).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcSuccess<R> {
    pub ok: bool,
    #[serde(default)]
    pub product: String,
    pub protocol_version: u64,
    #[serde(default)]
    pub request_id: String,
    pub result: R,
}

impl<R> RpcSuccess<R> {
    /// The TypeScript client rejects a success envelope whose version differs; so should a caller.
    pub fn protocol_version_matches(&self) -> bool {
        self.protocol_version == GXSERVER_PROTOCOL_VERSION
    }
}

/// Error envelope. `protocolVersion` and `requestId` are optional on errors.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcFailure {
    pub ok: bool,
    #[serde(default)]
    pub product: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub error: RpcErrorCode,
    #[serde(default)]
    pub message: String,
}

/// A parsed RPC response, discriminated by the envelope's `ok` flag.
#[derive(Clone, Debug, PartialEq)]
pub enum RpcResponse<R> {
    Success(RpcSuccess<R>),
    Failure(RpcFailure),
}

/// Why a response body could not be read as an RPC envelope.
#[derive(Debug)]
pub enum RpcEnvelopeError {
    /// The body is not JSON, or does not match the envelope for its `ok` value.
    Json(serde_json::Error),
    /// The body is JSON but has no boolean `ok`.
    MissingOk,
}

impl std::fmt::Display for RpcEnvelopeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "invalid gxserver RPC envelope: {error}"),
            Self::MissingOk => formatter.write_str("gxserver RPC envelope has no boolean `ok`"),
        }
    }
}

impl std::error::Error for RpcEnvelopeError {}

impl<R: DeserializeOwned> RpcResponse<R> {
    /// Parses a response body. The envelope is picked from `ok`, not from the HTTP status.
    pub fn parse(body: &str) -> Result<Self, RpcEnvelopeError> {
        let value: Value = serde_json::from_str(body).map_err(RpcEnvelopeError::Json)?;
        Self::from_value(value)
    }

    pub fn from_value(value: Value) -> Result<Self, RpcEnvelopeError> {
        match value.get("ok").and_then(Value::as_bool) {
            Some(true) => serde_json::from_value(value)
                .map(Self::Success)
                .map_err(RpcEnvelopeError::Json),
            Some(false) => serde_json::from_value(value)
                .map(Self::Failure)
                .map_err(RpcEnvelopeError::Json),
            None => Err(RpcEnvelopeError::MissingOk),
        }
    }
}
