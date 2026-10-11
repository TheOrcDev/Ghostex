//! Where Ghostex Capture keeps every picture it takes.
//!
//! CDXC:GhostexCapture 2026-09-30 DECISION:
//! User: "all of the images original and edited need to be saved to ~/documents/screenshots and the
//! screenshot folder on windows/linux". Windows' is Pictures\Screenshots (where Snipping Tool and
//! Win+PrintScreen save), Linux's is Screenshots inside the XDG pictures folder (GNOME's).
//!
//! File names carry no spaces: prompts reference them as `[Image #N](path)`, and a space in the
//! path would break that link for an agent reading it.

use std::path::{Path, PathBuf};

use image::RgbaImage;

fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(target_os = "windows") {
        "USERPROFILE"
    } else {
        "HOME"
    })
    .map(PathBuf::from)
}

/// `XDG_PICTURES_DIR` from `user-dirs.dirs`, else ~/Pictures.
#[cfg(target_os = "linux")]
fn pictures_dir(home: &Path) -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    std::fs::read_to_string(config.join("user-dirs.dirs"))
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                let value = line.trim().strip_prefix("XDG_PICTURES_DIR=")?;
                let value = value.trim().trim_matches('"');
                let expanded = value.replace("$HOME", &home.to_string_lossy());
                (!expanded.is_empty()).then(|| PathBuf::from(expanded))
            })
        })
        .unwrap_or_else(|| home.join("Pictures"))
}

pub(crate) fn screenshots_dir() -> Option<PathBuf> {
    let home = home()?;
    #[cfg(target_os = "macos")]
    let directory = home.join("Documents").join("Screenshots");
    #[cfg(target_os = "windows")]
    let directory = home.join("Pictures").join("Screenshots");
    #[cfg(target_os = "linux")]
    let directory = pictures_dir(&home).join("Screenshots");
    Some(directory)
}

/// A file name for a new picture: `Ghostex-Capture-2026-09-30-at-05.12.33.png`, with `-edited`
/// for a marked-up copy and a counter when the second is already taken.
fn next_path(directory: &Path, edited: bool) -> PathBuf {
    let stamp = chrono::Local::now()
        .format("%Y-%m-%d-at-%H.%M.%S")
        .to_string();
    let suffix = if edited { "-edited" } else { "" };
    let mut path = directory.join(format!("Ghostex-Capture-{stamp}{suffix}.png"));
    let mut counter = 2;
    while path.exists() {
        path = directory.join(format!("Ghostex-Capture-{stamp}{suffix}-{counter}.png"));
        counter += 1;
    }
    path
}

/// Writes `image` as a new PNG in the screenshots folder and returns its path.
pub(crate) fn save_png(image: &RgbaImage, edited: bool) -> Result<PathBuf, String> {
    let directory =
        screenshots_dir().ok_or_else(|| "Could not find your screenshots folder.".to_string())?;
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create {}: {error}", directory.display()))?;
    let path = next_path(&directory, edited);
    image
        .save_with_format(&path, image::ImageFormat::Png)
        .map_err(|error| format!("Could not save the screenshot: {error}"))?;
    Ok(path)
}
