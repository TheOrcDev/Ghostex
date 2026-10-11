//! A minimal HTTP/1.1 client for gxserver's loopback control API (health and stop). Only `http://127.0.0.1:<port>` is ever addressed, so no TLS or redirects.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

/// gxserver's control-plane protocol version header.
const PROTOCOL_VERSION: &str = "1";

/// Sends a request and returns the parsed JSON body of a 2xx response; None for any failure, as the old `fetch` wrapper did.
pub fn request_json(
    base_url: &str,
    method: &str,
    path: &str,
    token: &str,
    timeout: Duration,
) -> Option<serde_json::Value> {
    let deadline = Instant::now() + timeout;
    let host = base_url.strip_prefix("http://")?.trim_end_matches('/');
    let address: SocketAddr = host.parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&address, timeout).ok()?;
    stream.set_write_timeout(Some(remaining(deadline)?)).ok()?;
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {token}\r\nx-gxserver-protocol-version: {PROTOCOL_VERSION}\r\nAccept: application/json\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut response = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        stream.set_read_timeout(Some(remaining(deadline)?)).ok()?;
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => response.extend_from_slice(&buffer[..read]),
            Err(_) => return None,
        }
        if let Some(body) = complete_body(&response) {
            return parse(body?);
        }
    }
    parse(
        complete_body(&response)
            .flatten()
            .or_else(|| body_until_close(&response))?,
    )
}

fn remaining(deadline: Instant) -> Option<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
}

fn parse(body: Vec<u8>) -> Option<serde_json::Value> {
    serde_json::from_slice(&body).ok()
}

fn split_head(response: &[u8]) -> Option<(String, &[u8])> {
    let end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")?;
    Some((
        String::from_utf8_lossy(&response[..end]).into_owned(),
        &response[end + 4..],
    ))
}

fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.lines().skip(1).find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim().eq_ignore_ascii_case(name).then(|| value.trim())
    })
}

fn status_ok(head: &str) -> bool {
    head.lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .is_some_and(|code| code.starts_with('2'))
}

/// Some(Some(body)) when a complete 2xx body has arrived, Some(None) for a complete non-2xx response, None while incomplete.
fn complete_body(response: &[u8]) -> Option<Option<Vec<u8>>> {
    let (head, rest) = split_head(response)?;
    if !status_ok(&head) {
        return Some(None);
    }
    if header(&head, "transfer-encoding").is_some_and(|value| value.eq_ignore_ascii_case("chunked"))
    {
        return decode_chunked(rest).map(Some);
    }
    let length: usize = header(&head, "content-length")?.parse().ok()?;
    (rest.len() >= length).then(|| Some(rest[..length].to_vec()))
}

fn body_until_close(response: &[u8]) -> Option<Vec<u8>> {
    let (head, rest) = split_head(response)?;
    status_ok(&head).then(|| rest.to_vec())
}

fn decode_chunked(mut data: &[u8]) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let line_end = data.windows(2).position(|window| window == b"\r\n")?;
        let size_text = String::from_utf8_lossy(&data[..line_end]);
        let size = usize::from_str_radix(size_text.split(';').next()?.trim(), 16).ok()?;
        data = &data[line_end + 2..];
        if size == 0 {
            return Some(body);
        }
        if data.len() < size + 2 {
            return None;
        }
        body.extend_from_slice(&data[..size]);
        data = &data[size + 2..];
    }
}
