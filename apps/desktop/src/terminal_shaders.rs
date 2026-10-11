//! Ghostty custom shaders over the existing terminal's painted pixels.
//!
//! This module does not own a PTY, an input handler, or a native child view.
//! A failed shader disables the effect and leaves the ordinary terminal intact.

use std::{
    ffi::CString,
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

use chrono::{Datelike, Timelike};
use web_time::Instant;

use crate::{
    ghostty_kit::ffi,
    terminal_model::{Rgb, TerminalSnapshot},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShaderAnimation {
    Focused,
    Always,
    Off,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerminalShaderSettings {
    pub sources: Arc<[Arc<str>]>,
    pub animation: ShaderAnimation,
    pub enabled: bool,
    pub background_alpha: f32,
}

pub fn enabled() -> bool {
    crate::shared_settings::shared_sidebar_settings_snapshot()
        .object()
        .get("terminalShadersEnabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

type ShaderCacheKey = Vec<(PathBuf, Option<(SystemTime, u64)>)>;
type ShaderSources = Arc<[Arc<str>]>;

/// Called after Ghostty's one-time initialization and finalized config loading.
/// Reuse compiled sources across panes; failed files are retried only when their
/// metadata changes. The small cache also bounds storage after config reloads.
pub fn load(
    paths: Vec<PathBuf>,
    animation: &str,
    background_alpha: f32,
) -> Option<TerminalShaderSettings> {
    if !enabled() || paths.is_empty() {
        return None;
    }
    static CACHE: OnceLock<Mutex<Vec<(ShaderCacheKey, Option<ShaderSources>)>>> = OnceLock::new();
    let key = paths
        .iter()
        .map(|path| {
            let metadata = path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok().map(|time| (time, m.len())));
            (path.clone(), metadata)
        })
        .collect::<ShaderCacheKey>();
    let mut cache = CACHE.get_or_init(Default::default).lock().ok()?;
    let sources = if let Some((_, sources)) = cache.iter().find(|(cached, _)| cached == &key) {
        sources.clone()
    } else {
        let sources = paths
            .iter()
            .map(|path| {
                let path = CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
                // Ghostty owns both translation and allocation. Copy before freeing;
                // its returned string is never retained across the FFI boundary.
                let compiled = unsafe { ffi::ghostty_custom_shader_load_msl(path.as_ptr()) };
                let source = if compiled.ptr.is_null() {
                    None
                } else {
                    let bytes = unsafe {
                        std::slice::from_raw_parts(compiled.ptr.cast::<u8>(), compiled.len)
                    };
                    std::str::from_utf8(bytes).ok().map(Arc::<str>::from)
                };
                unsafe { ffi::ghostty_string_free(compiled) };
                source
            })
            .collect::<Option<Vec<_>>>()
            .map(ShaderSources::from);
        if sources.is_none() {
            eprintln!(
                "Terminal shaders could not be compiled; retaining the ordinary terminal renderer"
            );
        }
        if cache.len() == 8 {
            cache.remove(0);
        }
        cache.push((key, sources.clone()));
        sources
    }?;
    Some(TerminalShaderSettings {
        sources,
        background_alpha,
        enabled: enabled(),
        animation: match animation {
            "false" => ShaderAnimation::Off,
            "always" => ShaderAnimation::Always,
            _ => ShaderAnimation::Focused,
        },
    })
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct ShaderCursor {
    /// Ghostty's Metal convention: x is the left edge, y is the bottom edge.
    pub rect: [f32; 4],
    pub color: [f32; 4],
    pub style: i32,
}

pub struct TerminalShaderState {
    pub id: u64,
    started: Instant,
    last_frame: Instant,
    frame: i32,
    cursor: ShaderCursor,
    previous_cursor: ShaderCursor,
    cursor_initialized: bool,
    cursor_changed_at: f32,
    focused: bool,
    focused_at: f32,
    /// A new terminal snapshot arrived since the cached uniforms were built.
    snapshot_changed: bool,
    last_inputs: Option<ShaderInputs>,
    cached: Option<Arc<[u8]>>,
}

type ShaderInputs = ([f32; 2], Option<ShaderCursor>, bool);

impl Default for TerminalShaderState {
    fn default() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let now = Instant::now();
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            started: now,
            last_frame: now,
            frame: 0,
            cursor: ShaderCursor::default(),
            previous_cursor: ShaderCursor::default(),
            cursor_initialized: false,
            cursor_changed_at: 0.0,
            focused: false,
            focused_at: 0.0,
            snapshot_changed: true,
            last_inputs: None,
            cached: None,
        }
    }
}

impl TerminalShaderState {
    /// The terminal content changed, so the next paint advances the shader.
    pub fn invalidate(&mut self) {
        self.snapshot_changed = true;
    }

    /// Serialize Ghostty's std140 Globals block without reading Rust padding.
    /// Offsets mirror renderer/shaders/shadertoy_prefix.glsl; vec3 and array
    /// elements occupy 16-byte slots. Selection colors follow the GLSL order.
    pub fn uniforms(
        &mut self,
        resolution: [f32; 2],
        cursor: Option<ShaderCursor>,
        focused: bool,
        animate: bool,
        snapshot: &TerminalSnapshot,
    ) -> Arc<[u8]> {
        // GPUI repaints the whole window when any pane changes, so a pane that
        // is not animating would otherwise tick its iTime and iFrame on every
        // unrelated frame. Like Ghostty, it only advances when it has its own
        // reason to render; the clock stays wall-clock, so the next real frame
        // jumps to the current elapsed time.
        let inputs: ShaderInputs = (resolution, cursor, focused);
        if !animate
            && !self.snapshot_changed
            && self.last_inputs == Some(inputs)
            && let Some(cached) = &self.cached
        {
            return cached.clone();
        }
        self.snapshot_changed = false;
        self.last_inputs = Some(inputs);

        let now = Instant::now();
        let time = now.duration_since(self.started).as_secs_f32();
        let delta = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.frame = self.frame.saturating_add(1);
        if let Some(cursor) = cursor {
            if !self.cursor_initialized {
                self.cursor = cursor;
                self.previous_cursor = cursor;
                self.cursor_initialized = true;
            } else {
                // Pane focus changes solid/hollow style without moving the
                // cursor. Ghostty only restarts a trail for position or color.
                if cursor.rect != self.cursor.rect || cursor.color != self.cursor.color {
                    self.previous_cursor = self.cursor;
                    self.cursor = cursor;
                    self.cursor_changed_at = time;
                } else {
                    self.previous_cursor.style = self.cursor.style;
                    self.cursor.style = cursor.style;
                }
            }
        }
        if focused && !self.focused {
            self.focused_at = time;
        }
        self.focused = focused;

        let mut bytes = vec![0; 4496];
        let floats = |bytes: &mut [u8], offset: usize, values: &[f32]| {
            for (i, value) in values.iter().enumerate() {
                bytes[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&value.to_le_bytes());
            }
        };
        let integer = |bytes: &mut [u8], offset: usize, value: i32| {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        };
        floats(
            &mut bytes,
            0,
            &[
                resolution[0],
                resolution[1],
                1.0,
                time,
                delta,
                if delta > 0.0 { 1.0 / delta } else { 0.0 },
            ],
        );
        integer(&mut bytes, 24, self.frame);
        floats(&mut bytes, 32, &[time]);
        floats(&mut bytes, 96, &[resolution[0], resolution[1], 1.0, 0.0]);
        let date = chrono::Local::now();
        floats(
            &mut bytes,
            176,
            &[
                date.year() as f32,
                date.month0() as f32,
                date.day() as f32,
                date.num_seconds_from_midnight() as f32 + date.nanosecond() as f32 / 1e9,
            ],
        );
        floats(&mut bytes, 208, &self.cursor.rect);
        floats(&mut bytes, 224, &self.previous_cursor.rect);
        floats(&mut bytes, 240, &self.cursor.color);
        floats(&mut bytes, 256, &self.previous_cursor.color);
        integer(&mut bytes, 272, self.cursor.style);
        integer(&mut bytes, 276, self.previous_cursor.style);
        integer(&mut bytes, 280, i32::from(snapshot.cursor_visible));
        floats(&mut bytes, 284, &[self.cursor_changed_at, self.focused_at]);
        integer(&mut bytes, 292, i32::from(focused));
        for (index, color) in snapshot.palette.iter().enumerate() {
            floats(&mut bytes, 304 + index * 16, &rgba(*color));
        }
        floats(&mut bytes, 4400, &rgba(snapshot.background));
        floats(&mut bytes, 4416, &rgba(snapshot.foreground));
        floats(
            &mut bytes,
            4432,
            &rgba(snapshot.cursor_color.unwrap_or(snapshot.foreground)),
        );
        floats(&mut bytes, 4448, &rgba(snapshot.background));
        floats(&mut bytes, 4464, &rgba(snapshot.background));
        floats(&mut bytes, 4480, &rgba(snapshot.foreground));
        let bytes: Arc<[u8]> = bytes.into();
        self.cached = Some(bytes.clone());
        bytes
    }
}

fn rgba(color: Rgb) -> [f32; 4] {
    [
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        1.0,
    ]
}
