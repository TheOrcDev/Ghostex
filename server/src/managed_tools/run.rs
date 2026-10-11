//! Running an installer command from a blocking install job, streaming its output into the job.

use std::{
    ffi::OsStr,
    io::Read,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};

use super::jobs::Log;

/// The PATH a tool command runs with: `prefix` first (a tool installed moments ago), then the
/// normalized session PATH, which already ends with Ghostex's tool folders.
pub(crate) fn job_path(prefix: &[PathBuf]) -> std::ffi::OsString {
    #[cfg(windows)]
    let base = crate::platform::live_path::value();
    #[cfg(not(windows))]
    let base = crate::agent_hooks::probing::normalize_gxserver_process_path(
        std::env::var("PATH").ok().as_deref(),
        &ghostex_paths::GhostexPaths::resolve().home_dir,
    );
    let mut entries: Vec<PathBuf> = prefix.to_vec();
    entries.extend(std::env::split_paths(&base));
    std::env::join_paths(entries).unwrap_or_else(|_| base.into())
}

pub(crate) fn run(
    program: impl AsRef<OsStr>,
    args: &[&str],
    env: &[(&str, &OsStr)],
    prefix: &[PathBuf],
    log: &Log,
    timeout: Duration,
) -> Result<(), String> {
    let program = program.as_ref();
    let mut command = crate::platform::process::background_command(program);
    command
        .args(args)
        .env("PATH", job_path(prefix))
        .env("NO_COLOR", "1")
        .env("TERM", "dumb")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let name = Path::new(program)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start {name}: {error}"))?;
    let readers = [
        child.stdout.take().map(|pipe| drain(pipe, log.clone())),
        child.stderr.take().map(|pipe| drain(pipe, log.clone())),
    ];
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                #[cfg(unix)]
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{name} did not finish within {} minutes.",
                    timeout.as_secs() / 60
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(200)),
            Err(error) => return Err(error.to_string()),
        }
    };
    for reader in readers.into_iter().flatten() {
        let _ = reader.join();
    }
    if status.success() {
        Ok(())
    } else {
        Err(format!("{name} exited with {status}."))
    }
}

fn drain(mut pipe: impl Read + Send + 'static, log: Log) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        while let Ok(read) = pipe.read(&mut buffer) {
            if read == 0 {
                break;
            }
            log.chunk(String::from_utf8_lossy(&buffer[..read]).into_owned());
        }
    })
}
