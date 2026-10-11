use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose};
use percent_encoding::percent_decode_str;
use serde_json::{Value, json};
use tao::window::WindowId;
use uuid::Uuid;

use crate::*;

impl EditorApp {
    pub(crate) fn handle_paste_image(&mut self, window_id: WindowId, message: &Value) {
        let Some(request_id) = self.request_id_for_window(window_id) else {
            return;
        };
        let Some(session) = self.sessions.get(&request_id) else {
            return;
        };
        let paste_request_id = message
            .get("requestId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if paste_request_id.is_empty() {
            return;
        }

        let result = save_pasted_image(session, message);
        let detail = match result {
            Ok(path) => json!({
                "type": "imagePasteResult",
                "requestId": paste_request_id,
                "path": path,
            }),
            Err(error) => json!({
                "type": "imagePasteResult",
                "requestId": paste_request_id,
                "error": error,
            }),
        };
        if let Some(window) = self.windows.get(&window_id) {
            dispatch_host_message(window, &detail);
        }
    }

    pub(crate) fn handle_load_image_preview(&mut self, window_id: WindowId, message: &Value) {
        let preview_request_id = message
            .get("requestId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let Some(path) = message.get("path").and_then(Value::as_str) else {
            return;
        };
        if preview_request_id.is_empty() {
            return;
        }

        // The thumbnail shelf must load every image path already present in
        // the prompt text. Resolve short ~ paths natively and send data URLs
        // back to the web layer so webview local-file read limits do not
        // block thumbnail or popup rendering.
        let detail = match load_image_preview_data_url(path) {
            Ok(data_url) => json!({
                "type": "imagePreviewResult",
                "requestId": preview_request_id,
                "path": path,
                "dataUrl": data_url,
            }),
            Err(error) => json!({
                "type": "imagePreviewResult",
                "requestId": preview_request_id,
                "path": path,
                "error": error,
            }),
        };
        if let Some(window) = self.windows.get(&window_id) {
            dispatch_host_message(window, &detail);
        }
    }
}

fn load_image_preview_data_url(path: &str) -> Result<String, String> {
    let file_path = resolve_image_preview_path(path)
        .ok_or_else(|| "Image preview path does not point to a local image.".to_string())?;
    let mime_type = image_preview_mime_type(&file_path)
        .ok_or_else(|| "Image preview path does not point to a local image.".to_string())?;
    let data =
        fs::read(&file_path).map_err(|error| format!("Image preview read failed: {error}"))?;
    Ok(format!(
        "data:{mime_type};base64,{}",
        general_purpose::STANDARD.encode(data)
    ))
}

fn resolve_image_preview_path(path: &str) -> Option<PathBuf> {
    let trimmed = path.trim();
    if let Some(stripped) = trimmed.strip_prefix("file://") {
        let decoded = percent_decode_str(stripped).decode_utf8().ok()?;
        return Some(PathBuf::from(decoded.as_ref()));
    }
    if let Some(stripped) = trimmed.strip_prefix("~/") {
        let home = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE"))?;
        return Some(PathBuf::from(home).join(stripped));
    }
    let candidate = PathBuf::from(trimmed);
    candidate.is_absolute().then_some(candidate)
}

fn image_preview_mime_type(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "avif" => Some("image/avif"),
        "gif" => Some("image/gif"),
        "heic" | "heif" => Some("image/heic"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "svg" => Some("image/svg+xml"),
        "tif" | "tiff" => Some("image/tiff"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn save_pasted_image(session: &EditorSession, message: &Value) -> Result<PathBuf, String> {
    let base64_data = message
        .get("base64Data")
        .and_then(Value::as_str)
        .ok_or_else(|| "Image paste did not include base64Data.".to_string())?;
    let encoded = base64_data
        .split_once(',')
        .map(|(_, tail)| tail)
        .unwrap_or(base64_data);
    let data = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| "Image paste data was not valid base64.".to_string())?;
    let image_directory = ensure_image_directory(&session.file_path)
        .map_err(|error| format!("Unable to prepare image directory: {error}"))?;
    let suggested_name = message
        .get("suggestedName")
        .and_then(Value::as_str)
        .unwrap_or("image.png");
    let file_name = unique_image_file_name(suggested_name);
    let file_path = image_directory.join(file_name);
    fs::write(&file_path, data).map_err(|error| format!("Unable to write image: {error}"))?;
    Ok(file_path)
}

fn ensure_image_directory(file_path: &Path) -> io::Result<PathBuf> {
    let draft_directory = file_path.parent().unwrap_or_else(|| Path::new("."));
    let base_directory = if draft_directory.exists() {
        draft_directory.to_path_buf()
    } else {
        env::temp_dir()
    };
    let image_directory = base_directory.join("ghostex-editor-images");
    fs::create_dir_all(&image_directory)?;
    Ok(image_directory)
}

fn unique_image_file_name(suggested_name: &str) -> String {
    let base = Path::new(suggested_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image.png");
    let mut filtered: String = base
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    filtered = filtered.trim_matches(['.', '-', ' ']).to_string();
    if filtered.is_empty() {
        filtered = "image.png".to_string();
    }
    if Path::new(&filtered).extension().is_none() {
        filtered.push_str(".png");
    }
    format!("{}-{filtered}", Uuid::new_v4())
}
