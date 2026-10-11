use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, os::windows::ffi::OsStrExt, path::Path, sync::Mutex};
use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

static REFRESH_LOCK: Mutex<()> = Mutex::new(());
const BINARIES: [&str; 3] = ["gxserver.exe", "ghostex.exe", "wmx.exe"];

/// Records the installed app folder for CLIs and scripts that cannot derive it (see `GhostexPaths::windows_app_dir_file`).
pub(super) fn record_app_dir() {
    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    let Some(app_dir) = executable.parent() else {
        return;
    };
    // Unbundled development executables are not an installed app.
    if !app_dir.join("resources/native/gxserver.exe").is_file() {
        return;
    }
    let target = crate::shared_settings::ghostex_storage_paths().windows_app_dir_file();
    let contents = app_dir.to_string_lossy();
    if fs::read_to_string(&target).is_ok_and(|current| current == contents) {
        return;
    }
    let written = target
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(&target, contents.as_bytes()));
    if let Err(error) = written {
        eprintln!(
            "Ghostex could not record its install folder in {}: {error}",
            target.display()
        );
    }
}

/// CDXC:PlatformSupport 2026-09-23 WHY:
/// An older managed CLI can replace the app's current server with its own sibling server long after app startup.
/// Refresh the existing managed package from the installed bundle before connecting, retaining replaced images because Windows session hosts may still map them.
pub(super) fn refresh_existing() -> Result<(), String> {
    let _guard = REFRESH_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    refresh()
        .map_err(|error| format!("Could not update the managed Windows CLI package: {error:#}"))
}

fn refresh() -> Result<()> {
    let package = crate::shared_settings::ghostex_storage_paths()
        .gxserver_data_dir()
        .join("package");
    if !package.exists() {
        return Ok(());
    }
    let resources = std::env::current_exe()?
        .parent()
        .context("The app executable has no parent folder")?
        .join("resources");
    // Unbundled development executables do not own a packaged runtime.
    if !resources.join("native/gxserver.exe").is_file() {
        return Ok(());
    }
    let identity_bytes = fs::read(resources.join("build-identity.json"))?;
    let identity: serde_json::Value = serde_json::from_slice(&identity_bytes)?;
    let mut source_hashes = Vec::new();
    let mut files = Vec::new();
    for name in BINARIES {
        let source = resources.join("native").join(name);
        let target = package.join("bin").join(name);
        ensure!(
            target.is_file(),
            "The existing package is missing {}",
            target.display()
        );
        let hash = file_hash(&source)?;
        source_hashes.push(hash.clone());
        if file_hash(&target)? != hash {
            files.push((source, target, hash));
        }
    }
    let fingerprint = format!(
        "sha256:{:x}",
        Sha256::digest(format!("{}\n", source_hashes.join("\n")).as_bytes())
    );
    ensure!(
        identity["fingerprint"].as_str() == Some(&fingerprint),
        "The bundled runtime fingerprint does not match its executables"
    );
    let version = identity["packageVersion"]
        .as_str()
        .context("Missing runtime package version")?;
    ensure!(
        identity["buildIdentity"].as_str() == Some(&format!("gxserver:{version}:{fingerprint}")),
        "The bundled build identity is inconsistent"
    );
    let identity_target = package.join("build-identity.json");
    if fs::read(&identity_target)? != identity_bytes {
        files.push((
            resources.join("build-identity.json"),
            identity_target,
            format!("{:x}", Sha256::digest(&identity_bytes)),
        ));
    }
    if files.is_empty() {
        return Ok(());
    }

    let transaction = package
        .join(".retired-native")
        .join(uuid::Uuid::new_v4().to_string());
    let staged = transaction.join("new");
    let previous = transaction.join("previous");
    fs::create_dir_all(&staged)?;
    fs::create_dir_all(&previous)?;
    // Verify every staged byte before replacing the first public entrypoint.
    for (source, target, hash) in &files {
        let copy = staged.join(target.file_name().context("Missing package filename")?);
        fs::copy(source, &copy)?;
        fs::OpenOptions::new().write(true).open(&copy)?.sync_all()?;
        ensure!(
            file_hash(&copy)? == *hash,
            "Staged runtime verification failed: {}",
            copy.display()
        );
    }
    let mut replaced = Vec::new();
    for (_, target, _) in &files {
        let name = target.file_name().context("Missing package filename")?;
        let backup = previous.join(name);
        if let Err(error) = replace(target, &staged.join(name), Some(&backup)) {
            // ReplaceFile can have moved the original before a later step fails.
            if backup.exists() {
                replaced.push((target.clone(), backup));
            }
            let mut rollback_errors = Vec::new();
            for (target, backup) in replaced.iter().rev() {
                let restored = if target.exists() {
                    replace(target, backup, None)
                } else {
                    fs::rename(backup, target)
                };
                if let Err(error) = restored {
                    rollback_errors.push(format!("{}: {error}", target.display()));
                }
            }
            anyhow::bail!(
                "Replacing {} failed: {error}; rollback errors: {}",
                target.display(),
                rollback_errors.join(", ")
            );
        }
        replaced.push((target.clone(), backup));
    }
    Ok(())
}

fn file_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).with_context(|| format!("Reading {}", path.display()))?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            return Ok(format!("{:x}", hash.finalize()));
        }
        hash.update(&buffer[..count]);
    }
}

fn replace(target: &Path, staged: &Path, backup: Option<&Path>) -> std::io::Result<()> {
    let wide = |path: &Path| {
        path.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let target = wide(target);
    let staged = wide(staged);
    let backup = backup.map(wide);
    let succeeded = unsafe {
        ReplaceFileW(
            target.as_ptr(),
            staged.as_ptr(),
            backup
                .as_ref()
                .map_or(std::ptr::null(), |path| path.as_ptr()),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if succeeded == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
