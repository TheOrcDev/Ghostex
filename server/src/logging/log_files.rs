use super::*;

pub(super) fn write_gxserver_log_line(paths: &GxserverPaths, line: &str) -> Result<()> {
    let write_lock = log_file_write_lock(&paths.log_file);
    let _write_guard = write_lock.lock().expect("gxserver log writer poisoned");
    rotate_log_if_needed(&paths.log_file, line.len() as u64 + 1)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.log_file)
        .with_context(|| "open gxserver log file")?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub(super) fn log_file_write_lock(log_file: &Path) -> Arc<Mutex<()>> {
    let locks = LOG_FILE_WRITE_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = locks
        .lock()
        .expect("gxserver log writer lock registry poisoned");
    guard
        .entry(log_file.to_path_buf())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

fn rotate_log_if_needed(log_file: &Path, incoming_byte_count: u64) -> Result<()> {
    let size = fs::metadata(log_file)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if size + incoming_byte_count <= LOG_FILE_MAX_BYTES {
        return Ok(());
    }
    let _ = fs::remove_file(rotated_log_file(log_file, LOG_FILE_MAX_ROTATIONS));
    for index in (1..LOG_FILE_MAX_ROTATIONS).rev() {
        let source = rotated_log_file(log_file, index);
        let destination = rotated_log_file(log_file, index + 1);
        match fs::rename(&source, &destination) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).with_context(|| "rotate gxserver log"),
        }
    }
    match fs::rename(log_file, rotated_log_file(log_file, 1)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| "rotate gxserver log"),
    }
}

pub(super) fn rotated_log_file(log_file: &Path, index: usize) -> PathBuf {
    PathBuf::from(format!("{}.{}", log_file.display(), index))
}

pub(super) fn gxserver_log_files(log_file: &Path) -> Vec<PathBuf> {
    let mut files = vec![log_file.to_path_buf()];
    files.extend((1..=LOG_FILE_MAX_ROTATIONS).map(|index| rotated_log_file(log_file, index)));
    files
}

pub(super) fn retained_gxserver_log_file(
    active_log_file: &Path,
    log_files: &[PathBuf],
) -> Result<Option<PathBuf>> {
    if is_file_if_exists(active_log_file)? {
        return Ok(Some(active_log_file.to_path_buf()));
    }
    let mut retained: Option<(PathBuf, std::time::SystemTime)> = None;
    for log_file in log_files {
        let metadata = match fs::metadata(log_file) {
            Ok(metadata) if metadata.is_file() => metadata,
            Ok(_) => continue,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).with_context(|| "stat gxserver log rotation"),
        };
        let modified = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        if retained
            .as_ref()
            .map(|(_, retained_modified)| modified > *retained_modified)
            .unwrap_or(true)
        {
            retained = Some((log_file.clone(), modified));
        }
    }
    Ok(retained.map(|(path, _)| path))
}

fn is_file_if_exists(path: &Path) -> Result<bool> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| "stat gxserver log file"),
    }
}

pub(super) fn prune_log_file_to_max_lines_with_before_rewrite(
    log_file: &Path,
    max_lines: usize,
    before_rewrite: impl FnOnce(),
) -> Result<()> {
    if max_lines == 0 {
        return Ok(());
    }
    let content = match fs::read_to_string(log_file) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| "read gxserver log for retention"),
    };
    let mut lines = if content.ends_with('\n') {
        content[..content.len() - 1]
            .split('\n')
            .map(str::to_string)
            .collect::<Vec<_>>()
    } else {
        content.split('\n').map(str::to_string).collect::<Vec<_>>()
    };
    if lines.len() <= max_lines {
        return Ok(());
    }
    let start = lines.len() - max_lines;
    lines.drain(0..start);
    before_rewrite();
    fs::write(log_file, format!("{}\n", lines.join("\n")))
        .with_context(|| "write retained gxserver log lines")
}
