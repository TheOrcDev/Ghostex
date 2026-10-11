//! Opening the `/api/events` socket of the loopback daemon.

use std::net::{TcpStream, ToSocketAddrs};

use ghostex_gx_protocol::{
    GXSERVER_EVENTS_PATH, GXSERVER_PROTOCOL_VERSION, GXSERVER_PROTOCOL_VERSION_HEADER,
};
use tungstenite::client::client_with_config;
use tungstenite::http::Uri;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{ClientRequestBuilder, Error as SocketError, HandshakeError, WebSocket};

use crate::config::{
    CONNECT_TIMEOUT, HANDSHAKE_TIMEOUT, MAX_FRAME_BYTES, SOCKET_READ_TIMEOUT, SOCKET_WRITE_TIMEOUT,
};

pub(crate) type EventSocket = WebSocket<TcpStream>;

/// Host and port of a plain `http://host:port` base URL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Endpoint {
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl Endpoint {
    /// `None` for anything but `http://<host>:<port>` with an optional trailing slash: this client
    /// is for the loopback daemon and has no TLS.
    pub(crate) fn parse(base_url: &str) -> Option<Self> {
        let authority = base_url
            .trim()
            .strip_prefix("http://")?
            .trim_end_matches('/');
        if authority.is_empty() || authority.contains(['/', '?', '#', '@']) {
            return None;
        }
        let (host, port) = authority.rsplit_once(':')?;
        let port = port.parse().ok()?;
        (!host.is_empty()).then(|| Self {
            host: host.to_string(),
            port,
        })
    }

    pub(crate) fn http_url(&self, path: &str) -> String {
        format!("http://{}:{}{path}", self.host, self.port)
    }
}

/// Connects and completes the WebSocket handshake. The token travels in the `Authorization`
/// header so it never appears in a URL; the protocol version travels in its header too, which is
/// the first place the daemon's exact-match gate looks.
///
/// Errors are short fixed descriptions plus, at most, an HTTP status or an OS error: never the
/// token, and never a response body.
pub(crate) fn connect(endpoint: &Endpoint, auth_token: &str) -> Result<EventSocket, String> {
    let address = (endpoint.host.as_str(), endpoint.port)
        .to_socket_addrs()
        .map_err(|error| format!("could not resolve the daemon address: {error}"))?
        .next()
        .ok_or_else(|| "the daemon address resolved to nothing".to_string())?;
    let stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT)
        .map_err(|error| format!("could not reach the daemon: {error}"))?;
    let configure = |result: std::io::Result<()>| {
        result.map_err(|error| format!("could not configure the event socket: {error}"))
    };
    configure(stream.set_nodelay(true))?;
    configure(stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT)))?;
    configure(stream.set_write_timeout(Some(SOCKET_WRITE_TIMEOUT)))?;

    let uri: Uri = format!(
        "ws://{}:{}{GXSERVER_EVENTS_PATH}",
        endpoint.host, endpoint.port
    )
    .parse()
    .map_err(|_| "the daemon address is not a valid URL".to_string())?;
    let request = ClientRequestBuilder::new(uri)
        .with_header("Authorization", format!("Bearer {auth_token}"))
        .with_header(
            GXSERVER_PROTOCOL_VERSION_HEADER,
            GXSERVER_PROTOCOL_VERSION.to_string(),
        );
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_FRAME_BYTES))
        .max_frame_size(Some(MAX_FRAME_BYTES));
    let (socket, _response) =
        client_with_config(request, stream, Some(config)).map_err(|error| match error {
            HandshakeError::Interrupted(_) => "the event stream handshake timed out".to_string(),
            HandshakeError::Failure(SocketError::Http(response)) => format!(
                "the daemon refused the event stream with HTTP {}",
                response.status().as_u16()
            ),
            HandshakeError::Failure(error) => {
                format!("the event stream handshake failed: {error}")
            }
        })?;
    // From here on a read only blocks briefly, so the thread stays responsive to its host.
    configure(socket.get_ref().set_read_timeout(Some(SOCKET_READ_TIMEOUT)))?;
    Ok(socket)
}
