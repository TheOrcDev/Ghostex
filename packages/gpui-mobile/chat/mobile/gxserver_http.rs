//! The chat view's direct gxserver calls on a phone: the desktop's typed-operation request
//! (`helpers/board_gxserver/typed_operations.rs`, whose framing and response parsing are lifted
//! here) sent to the gxserver the host named at `init` instead of the one in the desktop's
//! bootstrap files. Plain HTTP on purpose: the phone reaches gxserver on `127.0.0.1` through
//! `adb reverse` or on the LAN, exactly as the chat socket does.
use std::io::{Read as _, Write as _};
use std::net::{TcpStream, ToSocketAddrs as _};
use std::time::Duration;

use serde_json::Value;

#[allow(dead_code)]
mod lifted {
    include!(concat!(env!("OUT_DIR"), "/gxserver_http.rs"));
}
use lifted::*;

/// `POST <path>` with the protocol envelope to the gxserver named at `init`, answered as
/// `(status, body)`.
pub(crate) fn post_typed_operation(
    path: &str,
    params: &Value,
    timeout: Duration,
) -> Result<(u16, String), String> {
    let endpoint = super::init::endpoint().ok_or("Ghostex is not connected yet.")?;
    post_typed_operation_to(&endpoint, path, params, timeout)
}

/// `POST <path>` with the protocol envelope to one computer's gxserver (a remote target is the
/// phone's forward to it), answered as `(status, body)`.
pub(crate) fn post_typed_operation_to(
    endpoint: &super::init::Endpoint,
    path: &str,
    params: &Value,
    timeout: Duration,
) -> Result<(u16, String), String> {
    if !path.starts_with("/api/") {
        return Err("Invalid gxserver API path.".to_string());
    }
    let address = endpoint
        .base_url
        .strip_prefix("http://")
        .ok_or("The phone's chat reaches gxserver over http:// only.")?
        .trim_end_matches('/')
        .to_string();
    let socket = address
        .to_socket_addrs()
        .map_err(|_| format!("gxserver address {address} does not resolve."))?
        .next()
        .ok_or_else(|| format!("gxserver address {address} does not resolve."))?;
    let mut stream = TcpStream::connect_timeout(&socket, timeout.min(Duration::from_secs(10)))
        .map_err(|_| format!("gxserver is not reachable on {address}."))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|_| "Could not configure gxserver read timeout.".to_string())?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|_| "Could not configure gxserver write timeout.".to_string())?;
    let body = serde_json::json!({
        "protocolVersion": GPUI_GXSERVER_PROTOCOL_VERSION,
        "params": params,
    })
    .to_string();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\n{GPUI_GXSERVER_PROTOCOL_HEADER}: {GPUI_GXSERVER_PROTOCOL_VERSION}\r\nContent-Length: {}\r\n\r\n{body}",
        endpoint.auth_token,
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|_| "Could not send gxserver request.".to_string())?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|_| "Could not read gxserver response.".to_string())?;
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "gxserver returned an invalid HTTP response.".to_string())?;
    let status_code = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .ok_or_else(|| "gxserver returned an invalid HTTP status.".to_string())?;
    Ok((status_code, gxserver_http_response_body(headers, body)?))
}
