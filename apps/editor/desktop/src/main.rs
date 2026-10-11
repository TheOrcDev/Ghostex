#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod assets;
mod daemon;
mod images;
mod requests;
mod session;
mod state_files;
mod types;
mod window;

use std::process;

use assets::*;
use daemon::*;
use session::*;
use state_files::*;
use types::*;
use window::*;

fn main() {
    if let Err(error) = run() {
        eprintln!("ghostex-editor: {error}");
        process::exit(2);
    }
}
