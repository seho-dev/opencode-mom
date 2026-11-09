use std::collections::VecDeque;
use std::fs::{self, File, Metadata};
use std::io::Write;
use std::path::{Path, PathBuf};

use time::OffsetDateTime;
use uuid::Uuid;

use super::types::{Filter, LogPage, LogQuery, LogRecord};
use crate::error::AppError;

pub(super) const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_FILES: usize = 5;
const MAX_LINE_BYTES: usize = 8192;
const MAX_CURSORS: usize = 32;

fn unavailable() -> AppError {
    AppError::configuration("Local log storage unavailable")
}

fn create_private(path: &Path) -> Result<File, AppError> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(|_| unavailable())
}

fn stale_cursor() -> AppError {
    AppError::validation("Log cursor expired; reload logs")
}

fn safe_metadata(metadata: &Metadata, directory: bool) -> bool {
    if metadata.file_type().is_symlink()
        || (directory && !metadata.is_dir())
        || (!directory && (!metadata.is_file() || metadata.len() > MAX_FILE_BYTES))
    {
        return false;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return false;
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // SAFETY: geteuid reads the effective UID and takes no pointers.
        if metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o777 != if directory { 0o700 } else { 0o600 }
        {
            return false;
        }
    }
    true
}

fn same_file(left: &Metadata, right: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        left.dev() == right.dev() && left.ino() == right.ino()
    }

    #[cfg(not(unix))]
    {
        left.created().ok() == right.created().ok()
    }
}

pub(super) fn ensure_directory(path: &Path) -> Result<(), AppError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !safe_metadata(&metadata, true) {
                return Err(unavailable());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(path.parent().ok_or_else(unavailable)?)
                .map_err(|_| unavailable())?;
            let builder = &mut fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }

            match builder.create(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(unavailable()),
            }

            if !safe_metadata(
                &fs::symlink_metadata(path).map_err(|_| unavailable())?,
                true,
            ) {
                return Err(unavailable());
            }
        }

        Err(_) => return Err(unavailable()),
    }

    Ok(())
}

#[derive(Clone)]
struct Entry {
    name: String,
    metadata: Metadata,
}

fn stamp(name: &str) -> Option<i128> {
    let value = name.strip_prefix("mom-")?.strip_suffix(".jsonl")?;
    let (timestamp, id) = value.split_once('-')?;
    if timestamp.len() != 39 || !timestamp.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Uuid::parse_str(id).ok()?;
    timestamp.parse().ok()
}

#[derive(Clone)]
struct Snapshot {
    epoch: Uuid,
    files: Vec<Entry>,
    file: usize,
    offset: usize,
    filter: Filter,
}

pub(super) struct Storage {
    directory: PathBuf,
    directory_metadata: Option<Metadata>,
    active: Option<(Entry, File, u64)>,
    epoch: Uuid,
    cursors: VecDeque<(Uuid, Snapshot)>,
    pub(super) max_bytes: u64,
}

impl Storage {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            directory_metadata: None,
            active: None,
            epoch: Uuid::new_v4(),
            cursors: VecDeque::new(),
            max_bytes: MAX_FILE_BYTES,
        }
    }

    pub fn directory(&mut self) -> Result<PathBuf, AppError> {
        ensure_directory(&self.directory)?;
        self.entries()?;

        Ok(self.directory.clone())
    }

    fn entries(&mut self) -> Result<Vec<Entry>, AppError> {
        let metadata = match fs::symlink_metadata(&self.directory) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if self.directory_metadata.is_some() {
                    return Err(unavailable());
                }

                return Ok(Vec::new());
            }

            Err(_) => return Err(unavailable()),
        };
        if !safe_metadata(&metadata, true)
            || self
                .directory_metadata
                .as_ref()
                .is_some_and(|previous| !same_file(previous, &metadata))
        {
            return Err(unavailable());
        }
        self.directory_metadata = Some(metadata);
        let mut entries = Vec::new();
        for (index, entry) in fs::read_dir(&self.directory)
            .map_err(|_| unavailable())?
            .enumerate()
        {
            if index >= 512 {
                return Err(unavailable());
            }

            let entry = entry.map_err(|_| unavailable())?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !name.starts_with("mom-") || !name.ends_with(".jsonl") {
                continue;
            }

            if stamp(name).is_none() {
                return Err(unavailable());
            }

            let metadata = fs::symlink_metadata(entry.path()).map_err(|_| unavailable())?;
            if !safe_metadata(&metadata, false) {
                return Err(unavailable());
            }
            entries.push(Entry {
                name: name.to_owned(),
                metadata,
            });
        }
        entries.sort_unstable_by(|left, right| right.name.cmp(&left.name));

        Ok(entries)
    }

    pub fn start(&mut self) -> Result<(), AppError> {
        self.stop()?;
        ensure_directory(&self.directory)?;
        self.rotate()
    }

    fn rotate(&mut self) -> Result<(), AppError> {
        self.stop()?;
        let entries = self.entries()?;
        self.epoch = Uuid::new_v4();
        self.cursors.clear();

        for entry in entries.iter().skip(MAX_FILES - 1) {
            fs::remove_file(self.directory.join(&entry.name)).map_err(|_| unavailable())?;
        }

        let timestamp = OffsetDateTime::now_utc().unix_timestamp_nanos().max(
            entries
                .first()
                .and_then(|entry| stamp(&entry.name))
                .unwrap_or(0)
                .saturating_add(1),
        );
        let name = format!("mom-{timestamp:039}-{}.jsonl", Uuid::new_v4());
        let file = create_private(&self.directory.join(&name))?;
        let metadata = file.metadata().map_err(|_| unavailable())?;
        if !safe_metadata(&metadata, false) {
            return Err(unavailable());
        }
        self.active = Some((Entry { name, metadata }, file, 0));

        Ok(())
    }

    pub fn write(&mut self, record: &LogRecord) -> Result<(), AppError> {
        if !record.is_safe() {
            return Err(AppError::validation("Invalid diagnostic record"));
        }

        let mut bytes = serde_json::to_vec(record).map_err(|_| unavailable())?;
        bytes.push(b'\n');
        if bytes.len() > MAX_LINE_BYTES || bytes.len() as u64 > self.max_bytes {
            return Err(AppError::validation("Diagnostic record too large"));
        }

        let Some((_, _, size)) = &self.active else {
            return Err(unavailable());
        };
        if size.saturating_add(bytes.len() as u64) > self.max_bytes {
            self.rotate()?;
        }
        self.entries()?;
        let (entry, file, size) = self.active.as_mut().ok_or_else(unavailable)?;
        let path_metadata =
            fs::symlink_metadata(self.directory.join(&entry.name)).map_err(|_| unavailable())?;
        if !safe_metadata(&path_metadata, false)
            || !same_file(&entry.metadata, &path_metadata)
            || path_metadata.len() != *size
        {
            return Err(unavailable());
        }
        file.write_all(&bytes).map_err(|_| unavailable())?;
        *size += bytes.len() as u64;

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), AppError> {
        if let Some((_, mut file, _)) = self.active.take() {
            file.flush()
                .and_then(|()| file.sync_all())
                .map_err(|_| unavailable())?;
        }

        Ok(())
    }

    pub fn clear(&mut self, resume: bool) -> Result<(), AppError> {
        self.stop()?;
        let entries = self.entries()?;
        self.epoch = Uuid::new_v4();
        self.cursors.clear();

        for entry in entries {
            fs::remove_file(self.directory.join(entry.name)).map_err(|_| unavailable())?;
        }

        if resume {
            self.start()?;
        }

        Ok(())
    }

    pub fn query(&mut self, query: LogQuery) -> Result<LogPage, AppError> {
        let limit = query.limit.unwrap_or(100);
        if limit == 0 || limit > 200 {
            return Err(AppError::validation("Log limit must be between 1 and 200"));
        }

        let filter = Filter::new(&query)?;
        let entries = self.entries()?;
        if entries.len() > MAX_FILES {
            return Err(unavailable());
        }

        let mut snapshot = if let Some(cursor) = query.cursor {
            if cursor.len() > 36 {
                return Err(stale_cursor());
            }

            let id = Uuid::parse_str(&cursor).map_err(|_| stale_cursor())?;
            let snapshot = self
                .cursors
                .iter()
                .find(|(cursor, _)| *cursor == id)
                .map(|(_, snapshot)| snapshot.clone())
                .ok_or_else(stale_cursor)?;
            if snapshot.epoch != self.epoch
                || snapshot.filter != filter
                || snapshot.files.len() != entries.len()
                || !snapshot
                    .files
                    .iter()
                    .zip(&entries)
                    .all(|(previous, current)| {
                        previous.name == current.name
                            && previous.metadata.len() <= current.metadata.len()
                            && same_file(&previous.metadata, &current.metadata)
                            && (previous.metadata.len() < current.metadata.len()
                                || previous.metadata.modified().ok()
                                    == current.metadata.modified().ok())
                    })
            {
                return Err(stale_cursor());
            }
            snapshot
        } else {
            Snapshot {
                epoch: self.epoch,
                offset: entries
                    .first()
                    .map_or(0, |entry| entry.metadata.len() as usize),
                files: entries,
                file: 0,
                filter,
            }
        };
        let mut records = Vec::new();
        let mut invalid_lines = 0;
        while snapshot.file < snapshot.files.len() && records.len() < limit {
            let entry = &snapshot.files[snapshot.file];
            let path = self.directory.join(&entry.name);
            let metadata = fs::symlink_metadata(&path).map_err(|_| unavailable())?;
            if !safe_metadata(&metadata, false) || !same_file(&entry.metadata, &metadata) {
                return Err(stale_cursor());
            }

            let bytes = crate::private_file::read(&path, MAX_FILE_BYTES)
                .map_err(|_| unavailable())?
                .ok_or_else(unavailable)?;
            if bytes.len() < entry.metadata.len() as usize {
                return Err(stale_cursor());
            }

            while snapshot.offset > 0 && records.len() < limit {
                let end = snapshot.offset;
                let terminated = bytes[end - 1] == b'\n';
                let content_end = if terminated { end - 1 } else { end };
                let start = bytes[..content_end]
                    .iter()
                    .rposition(|byte| *byte == b'\n')
                    .map_or(0, |position| position + 1);
                snapshot.offset = start;
                let line = &bytes[start..content_end];
                if !terminated || line.len() > MAX_LINE_BYTES {
                    invalid_lines += 1;
                    continue;
                }

                match serde_json::from_slice::<LogRecord>(line) {
                    Ok(record) if record.is_safe() => {
                        if snapshot.filter.matches(&record) {
                            records.push(record);
                        }
                    }
                    _ => invalid_lines += 1,
                }
            }

            if snapshot.offset == 0 {
                snapshot.file += 1;
                snapshot.offset = snapshot
                    .files
                    .get(snapshot.file)
                    .map_or(0, |entry| entry.metadata.len() as usize);
            }
        }

        let next_cursor = if snapshot.file < snapshot.files.len() {
            let id = Uuid::new_v4();
            if self.cursors.len() >= MAX_CURSORS {
                self.cursors.pop_front();
            }
            self.cursors.push_back((id, snapshot));

            Some(id.to_string())
        } else {
            None
        };
        Ok(LogPage {
            records,
            next_cursor,
            invalid_lines,
        })
    }
}
