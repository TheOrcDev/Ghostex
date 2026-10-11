use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    path::Path,
};

use flate2::read::GzDecoder;

const CODE_SERVER_TAR_BLOCK_SIZE: usize = 512;
const CODE_SERVER_TAR_METADATA_LIMIT: u64 = 1024 * 1024;

fn raw_code_server_tar_string<'a>(field: &'a [u8], label: &str) -> Result<&'a str, String> {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    if field[end..].iter().any(|byte| *byte != 0) {
        return Err(format!("Malformed code-server tar {label}"));
    }
    std::str::from_utf8(&field[..end])
        .map_err(|_| format!("Invalid UTF-8 in code-server tar {label}"))
}

fn raw_code_server_tar_octal(field: &[u8], label: &str) -> Result<u64, String> {
    let start = field
        .iter()
        .position(|byte| !matches!(byte, 0 | b' '))
        .unwrap_or(field.len());
    let end = field[start..]
        .iter()
        .position(|byte| matches!(byte, 0 | b' '))
        .map(|offset| start + offset)
        .unwrap_or(field.len());
    if field[end..].iter().any(|byte| !matches!(byte, 0 | b' ')) {
        return Err(format!("Invalid code-server tar {label}"));
    }
    let value = &field[start..end];
    if value.is_empty() || !value.iter().all(|byte| matches!(byte, b'0'..=b'7')) {
        return Err(format!("Invalid code-server tar {label}"));
    }
    let value =
        std::str::from_utf8(value).map_err(|_| format!("Invalid code-server tar {label}"))?;
    u64::from_str_radix(value, 8).map_err(|_| format!("Invalid code-server tar {label}"))
}

fn raw_code_server_tar_header_name(
    header: &[u8; CODE_SERVER_TAR_BLOCK_SIZE],
) -> Result<String, String> {
    let name = raw_code_server_tar_string(&header[..100], "entry name")?;
    let prefix = raw_code_server_tar_string(&header[345..500], "entry prefix")?;
    Ok(if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    })
}

fn validate_raw_code_server_tar_checksum(
    header: &[u8; CODE_SERVER_TAR_BLOCK_SIZE],
) -> Result<(), String> {
    let stored = raw_code_server_tar_octal(&header[148..156], "header checksum")?;
    let actual = header
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            if (148..156).contains(&index) {
                u64::from(b' ')
            } else {
                u64::from(*byte)
            }
        })
        .sum::<u64>();
    if stored != actual {
        return Err("Invalid code-server tar header checksum".to_string());
    }
    Ok(())
}

fn parse_raw_code_server_pax_metadata(contents: &[u8]) -> Result<HashMap<String, String>, String> {
    let mut values = HashMap::new();
    let mut offset = 0_usize;
    while offset < contents.len() {
        let relative_space = contents[offset..]
            .iter()
            .position(|byte| *byte == b' ')
            .ok_or_else(|| "Malformed code-server PAX header".to_string())?;
        let space = offset + relative_space;
        let length_text = std::str::from_utf8(&contents[offset..space])
            .map_err(|_| "Malformed code-server PAX header".to_string())?;
        if length_text.is_empty()
            || length_text.starts_with('0')
            || !length_text.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("Malformed code-server PAX header".to_string());
        }
        let length = length_text
            .parse::<usize>()
            .map_err(|_| "Malformed code-server PAX header".to_string())?;
        let end = offset
            .checked_add(length)
            .filter(|end| *end <= contents.len())
            .ok_or_else(|| "Malformed code-server PAX header".to_string())?;
        if end <= space + 1 || contents[end - 1] != b'\n' {
            return Err("Malformed code-server PAX header".to_string());
        }
        let record = std::str::from_utf8(&contents[space + 1..end - 1])
            .map_err(|_| "Invalid UTF-8 in code-server PAX record".to_string())?;
        let (key, value) = record
            .split_once('=')
            .filter(|(key, value)| !key.is_empty() && !value.is_empty())
            .ok_or_else(|| "Malformed code-server PAX header".to_string())?;
        if !matches!(key, "path" | "linkpath") {
            return Err(format!("Unsupported code-server PAX field: {key}"));
        }
        if values.insert(key.to_string(), value.to_string()).is_some() {
            return Err(format!("Duplicate code-server PAX field: {key}"));
        }
        offset = end;
    }
    if values.is_empty() {
        return Err("Malformed empty code-server PAX header".to_string());
    }
    Ok(values)
}

fn raw_code_server_gnu_metadata(contents: &[u8], label: &str) -> Result<String, String> {
    let Some((&0, value)) = contents.split_last() else {
        return Err(format!("Malformed GNU tar {label}"));
    };
    if value.is_empty() || value.contains(&0) {
        return Err(format!("Malformed GNU tar {label}"));
    }
    std::str::from_utf8(value)
        .map(str::to_string)
        .map_err(|_| format!("Invalid UTF-8 in GNU tar {label}"))
}

fn read_raw_code_server_tar_payload(
    decoder: &mut GzDecoder<File>,
    size: u64,
    retain: bool,
) -> Result<Vec<u8>, String> {
    let padded_size = size
        .checked_add((CODE_SERVER_TAR_BLOCK_SIZE - 1) as u64)
        .map(|value| value / CODE_SERVER_TAR_BLOCK_SIZE as u64 * CODE_SERVER_TAR_BLOCK_SIZE as u64)
        .ok_or_else(|| "Invalid code-server tar entry size".to_string())?;
    if retain {
        let padded_size = usize::try_from(padded_size)
            .map_err(|_| "Invalid code-server tar metadata size".to_string())?;
        let size = usize::try_from(size)
            .map_err(|_| "Invalid code-server tar metadata size".to_string())?;
        let mut payload = vec![0_u8; padded_size];
        decoder
            .read_exact(&mut payload)
            .map_err(|error| format!("Truncated code-server tar metadata: {error}"))?;
        payload.truncate(size);
        return Ok(payload);
    }
    let copied = io::copy(&mut decoder.take(padded_size), &mut io::sink())
        .map_err(|error| format!("Could not inspect code-server tar payload: {error}"))?;
    if copied != padded_size {
        return Err("Truncated code-server tar payload".to_string());
    }
    Ok(Vec::new())
}

pub(crate) fn preflight_code_server_tar_metadata(archive_path: &Path) -> Result<(), String> {
    let file = File::open(archive_path).map_err(|error| {
        format!(
            "Could not open verified code-server archive {}: {error}",
            archive_path.display()
        )
    })?;
    let mut decoder = GzDecoder::new(file);
    let mut pending_long_name = None::<String>;
    let mut pending_long_link = None::<String>;
    let mut pending_pax = None::<HashMap<String, String>>;
    loop {
        let mut header = [0_u8; CODE_SERVER_TAR_BLOCK_SIZE];
        decoder
            .read_exact(&mut header)
            .map_err(|error| format!("Truncated code-server tar header: {error}"))?;
        if header.iter().all(|byte| *byte == 0) {
            if pending_long_name.is_some() || pending_long_link.is_some() || pending_pax.is_some() {
                return Err("Dangling code-server tar metadata header".to_string());
            }
            let mut trailing = Vec::new();
            decoder
                .read_to_end(&mut trailing)
                .map_err(|error| format!("Invalid code-server tar terminator: {error}"))?;
            if trailing.len() < CODE_SERVER_TAR_BLOCK_SIZE || trailing.iter().any(|byte| *byte != 0)
            {
                return Err("Malformed code-server tar terminator".to_string());
            }
            return Ok(());
        }

        validate_raw_code_server_tar_checksum(&header)?;
        let archive_name = raw_code_server_tar_header_name(&header)?;
        let entry_type = match header[156] {
            0 => b'0',
            value => value,
        };
        let size = raw_code_server_tar_octal(&header[124..136], "entry size")?;
        let normalized_archive_name =
            normalized_code_server_tar_path_text(&archive_name, entry_type == b'5')?;

        if matches!(entry_type, b'L' | b'K' | b'x') {
            if size == 0 || size > CODE_SERVER_TAR_METADATA_LIMIT {
                return Err(format!(
                    "Malformed code-server tar metadata size for {}",
                    normalized_archive_name.as_deref().unwrap_or(".")
                ));
            }
            let contents = read_raw_code_server_tar_payload(&mut decoder, size, true)?;
            match entry_type {
                b'L' => {
                    if pending_long_name.is_some() {
                        return Err("Duplicate GNU tar long-name header".to_string());
                    }
                    if pending_pax
                        .as_ref()
                        .is_some_and(|pax| pax.contains_key("path"))
                    {
                        return Err("Conflicting code-server tar path metadata".to_string());
                    }
                    if archive_name != "././@LongLink" {
                        return Err("Malformed GNU tar long-name header".to_string());
                    }
                    pending_long_name = Some(raw_code_server_gnu_metadata(&contents, "long name")?);
                }
                b'K' => {
                    if pending_long_link.is_some() {
                        return Err("Duplicate GNU tar long-link header".to_string());
                    }
                    if pending_pax
                        .as_ref()
                        .is_some_and(|pax| pax.contains_key("linkpath"))
                    {
                        return Err("Conflicting code-server tar link metadata".to_string());
                    }
                    if archive_name != "././@LongLink" {
                        return Err("Malformed GNU tar long-link header".to_string());
                    }
                    pending_long_link = Some(raw_code_server_gnu_metadata(&contents, "long link")?);
                }
                b'x' => {
                    if pending_pax.is_some() {
                        return Err("Duplicate local PAX tar header".to_string());
                    }
                    let metadata_path = normalized_archive_name.as_deref().ok_or_else(|| {
                        "Malformed local PAX code-server archive header".to_string()
                    })?;
                    if !metadata_path
                        .split('/')
                        .any(|segment| segment == "PaxHeader")
                    {
                        return Err("Malformed local PAX code-server archive header".to_string());
                    }
                    let pax = parse_raw_code_server_pax_metadata(&contents)?;
                    if pax.contains_key("path") && pending_long_name.is_some() {
                        return Err("Conflicting code-server tar path metadata".to_string());
                    }
                    if pax.contains_key("linkpath") && pending_long_link.is_some() {
                        return Err("Conflicting code-server tar link metadata".to_string());
                    }
                    pending_pax = Some(pax);
                }
                _ => unreachable!(),
            }
            continue;
        }
        if entry_type == b'g' {
            return Err("Unsupported global PAX code-server archive header".to_string());
        }
        if !matches!(entry_type, b'0' | b'1' | b'2' | b'5') {
            return Err(format!(
                "Unsupported code-server archive entry type {entry_type:#04x} for {}",
                normalized_archive_name.as_deref().unwrap_or(".")
            ));
        }
        if (pending_long_link.is_some()
            || pending_pax
                .as_ref()
                .is_some_and(|pax| pax.contains_key("linkpath")))
            && !matches!(entry_type, b'1' | b'2')
        {
            return Err("Code-server tar link metadata does not describe a link entry".to_string());
        }
        let effective_name = pending_pax
            .as_ref()
            .and_then(|pax| pax.get("path"))
            .or(pending_long_name.as_ref())
            .map(String::as_str)
            .unwrap_or(&archive_name);
        let effective_path =
            normalized_code_server_tar_path_text(effective_name, entry_type == b'5')?;
        if matches!(entry_type, b'1' | b'2') {
            let effective_path = effective_path
                .as_deref()
                .ok_or_else(|| "Malformed code-server archive link entry".to_string())?;
            let header_link = raw_code_server_tar_string(&header[157..257], "link target")?;
            let effective_link = pending_pax
                .as_ref()
                .and_then(|pax| pax.get("linkpath"))
                .or(pending_long_link.as_ref())
                .map(String::as_str)
                .unwrap_or(header_link);
            normalized_code_server_link_target(
                effective_path,
                Path::new(effective_link),
                entry_type == b'1',
            )?;
        }
        pending_long_name = None;
        pending_long_link = None;
        pending_pax = None;
        read_raw_code_server_tar_payload(&mut decoder, size, false)?;
    }
}

fn normalized_code_server_tar_path_text(
    path: &str,
    allow_root: bool,
) -> Result<Option<String>, String> {
    let mut normalized = path;
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped;
    }
    while let Some(stripped) = normalized.strip_suffix('/') {
        normalized = stripped;
    }
    if normalized.is_empty() && allow_root {
        return Ok(None);
    }
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.as_bytes().get(1) == Some(&b':')
        || normalized.contains('\\')
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        || normalized.chars().any(|character| character.is_control())
    {
        return Err("Code-server archive contains an unsafe entry".to_string());
    }
    Ok(Some(normalized.to_string()))
}

pub(crate) fn normalized_code_server_tar_path(
    path: &Path,
    allow_root: bool,
) -> Result<Option<String>, String> {
    normalized_code_server_tar_path_text(
        path.to_str()
            .ok_or_else(|| "Code-server archive contains a non-UTF-8 entry".to_string())?,
        allow_root,
    )
}

pub(crate) fn register_code_server_archive_entry(
    seen: &mut HashMap<String, &'static str>,
    path: &str,
    kind: &'static str,
) -> Result<(), String> {
    if seen.contains_key(path) {
        return Err(format!("Duplicate code-server archive entry: {path}"));
    }
    let segments = path.split('/').collect::<Vec<_>>();
    for index in 1..segments.len() {
        let parent = segments[..index].join("/");
        if seen
            .get(&parent)
            .is_some_and(|parent_kind| *parent_kind != "directory")
        {
            return Err(format!(
                "Conflicting code-server archive entries: {parent} and {path}"
            ));
        }
    }
    if kind != "directory"
        && seen
            .keys()
            .any(|entry| entry.starts_with(&format!("{path}/")))
    {
        return Err(format!(
            "Conflicting code-server archive entries beneath {path}"
        ));
    }
    seen.insert(path.to_string(), kind);
    Ok(())
}

pub(crate) fn normalized_code_server_link_target(
    entry_path: &str,
    link_path: &Path,
    hardlink: bool,
) -> Result<String, String> {
    if link_path.as_os_str().is_empty() {
        return Err("Code-server archive contains an unsafe empty link target".to_string());
    }
    if hardlink {
        return normalized_code_server_tar_path(link_path, false)?
            .ok_or_else(|| "Code-server archive contains an unsafe hardlink target".to_string());
    }
    let mut segments = entry_path
        .split('/')
        .take(entry_path.split('/').count().saturating_sub(1))
        .map(str::to_string)
        .collect::<Vec<_>>();
    for component in link_path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if segments.pop().is_none() {
                    return Err("Code-server archive contains an unsafe symlink target".to_string());
                }
            }
            std::path::Component::Normal(segment) => segments.push(
                segment
                    .to_str()
                    .ok_or_else(|| {
                        "Code-server archive contains a non-UTF-8 symlink target".to_string()
                    })?
                    .to_string(),
            ),
            _ => {
                return Err("Code-server archive contains an unsafe symlink target".to_string());
            }
        }
    }
    if segments.is_empty() {
        return Err("Code-server archive contains an unsafe symlink target".to_string());
    }
    Ok(segments.join("/"))
}
