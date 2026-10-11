use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use percent_encoding::percent_decode_str;
use wry::http::{Request, Response, header::CONTENT_TYPE};

pub(crate) fn asset_response(
    web_root: &Path,
    request: Request<Vec<u8>>,
) -> Response<std::borrow::Cow<'static, [u8]>> {
    match asset_response_body(web_root, request) {
        Ok((mime, body)) => Response::builder()
            .header(CONTENT_TYPE, mime)
            .body(std::borrow::Cow::Owned(body))
            .unwrap(),
        Err((status, message)) => Response::builder()
            .status(status)
            .header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(std::borrow::Cow::Owned(message.into_bytes()))
            .unwrap(),
    }
}

fn asset_response_body(
    web_root: &Path,
    request: Request<Vec<u8>>,
) -> Result<(&'static str, Vec<u8>), (u16, String)> {
    let relative_path = request.uri().path().trim_start_matches('/');
    let relative_path = if relative_path.is_empty() {
        "index.html"
    } else {
        relative_path
    };
    let decoded = percent_decode_str(relative_path)
        .decode_utf8()
        .map_err(|_| (400, "invalid asset path".to_string()))?;
    let path = safe_asset_path(web_root, &decoded)?;
    let body = fs::read(&path).map_err(|_| (404, "asset not found".to_string()))?;
    let mime = mime_guess::from_path(&path)
        .first_raw()
        .unwrap_or("application/octet-stream");
    Ok((mime, body))
}

fn safe_asset_path(web_root: &Path, relative: &str) -> Result<PathBuf, (u16, String)> {
    let mut clean = PathBuf::new();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            _ => return Err((403, "invalid asset path".to_string())),
        }
    }
    let path = web_root.join(clean);
    let canonical_root = web_root
        .canonicalize()
        .map_err(|_| (500, "web root unavailable".to_string()))?;
    let canonical_path = path
        .canonicalize()
        .map_err(|_| (404, "asset not found".to_string()))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err((403, "invalid asset path".to_string()));
    }
    Ok(canonical_path)
}
