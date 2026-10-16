use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use std::os::windows::fs::MetadataExt;

use crate::cleanup::model::Failure;
use crate::common::error::{AppError, Result};

use crate::common::path as fspath;

const DRY_RUN_VARS: &[&str] = &["PURIFIER_DRY_RUN", "P_URI_DRY_RUN"];
const KEEP_ROOT_VARS: &[&str] = &["PURIFIER_KEEP_ROOT", "P_URI_KEEP_ROOT"];

pub(crate) fn truthy(value: &str) -> bool {
    !matches!(
        value.trim().to_lowercase().as_str(),
        "" | "0" | "false" | "off" | "no"
    )
}

fn any_flag_set(vars: &[&str]) -> bool {
    vars.iter()
        .any(|name| std::env::var(name).is_ok_and(|value| truthy(&value)))
}

pub fn dry_run() -> bool {
    any_flag_set(DRY_RUN_VARS)
}

pub fn keep_root() -> bool {
    any_flag_set(KEEP_ROOT_VARS)
}

#[derive(Debug, Clone, Default)]
pub struct Counter(Option<Arc<AtomicU64>>);

impl Counter {
    pub fn shared() -> Self {
        Self(Some(Arc::new(AtomicU64::new(0))))
    }

    #[cfg(test)]
    pub fn none() -> Self {
        Self(None)
    }

    fn tick(&self) {
        if let Some(counter) = &self.0 {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn count(&self) -> u64 {
        self.0
            .as_ref()
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

#[derive(Debug, Default)]
pub struct Removal {
    pub freed_bytes: u64,
    pub failures: Vec<Failure>,
    pub entries: u64,
    pub retries: u32,
    pub slept_ms: u64,
    pub blocked: u64,
    pub files_removed: u64,
    pub elapsed_ms: u64,
}

impl Removal {
    fn merge(&mut self, other: Removal) {
        self.freed_bytes = self.freed_bytes.saturating_add(other.freed_bytes);
        self.failures.extend(other.failures);
        self.entries = self.entries.saturating_add(other.entries);
        self.retries = self.retries.saturating_add(other.retries);
        self.slept_ms = self.slept_ms.saturating_add(other.slept_ms);
        self.blocked = self.blocked.saturating_add(other.blocked);
        self.files_removed = self.files_removed.saturating_add(other.files_removed);
    }
}

fn fail(out: &mut Removal, path: &Path, err: &io::Error) {
    if is_permission_like(err) {
        out.blocked += 1;
    }
    out.failures.push(Failure {
        path: path.display().to_string(),
        reason: io_reason(err),
    });
}

fn io_reason(err: &io::Error) -> String {
    match err.raw_os_error() {
        Some(32) => "in use by another process".to_string(),
        Some(5) => "access denied".to_string(),
        Some(3) => "path not found".to_string(),
        _ => err.to_string(),
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn SetFileAttributesW(lpFileName: *const u16, dwFileAttributes: u32) -> i32;
}

const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;

fn make_writable(path: &Path) {
    let attrs = match std::fs::metadata(path) {
        Ok(meta) => meta.file_attributes(),
        Err(_) => return,
    };
    if attrs == FILE_ATTRIBUTE_NORMAL {
        return;
    }
    let mut wide: Vec<u16> = fspath::verbatim(path).as_os_str().encode_wide().collect();
    wide.push(0);
    // SAFETY: `wide` is NUL-terminated and outlives the call.
    unsafe {
        SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_NORMAL);
    }
}

fn is_permission_like(err: &io::Error) -> bool {
    matches!(err.kind(), io::ErrorKind::PermissionDenied)
        || matches!(err.raw_os_error(), Some(32) | Some(5))
}

const RETRY_BACKOFF_MS: [u64; 2] = [10, 25];

pub(crate) const RETRY_ATTEMPTS: usize = RETRY_BACKOFF_MS.len() + 1;

pub(crate) fn retry<F>(out: &mut Removal, mut op: F) -> io::Result<()>
where
    F: FnMut() -> io::Result<()>,
{
    let mut last = io::Error::other("operation never attempted");

    for attempt in 0..RETRY_ATTEMPTS {
        match op() {
            Ok(()) => return Ok(()),
            Err(err) => {
                let recoverable = is_permission_like(&err);
                last = err;
                match backoff_after(attempt) {
                    Some(wait) if recoverable => {
                        out.retries = out.retries.saturating_add(1);
                        out.slept_ms = out
                            .slept_ms
                            .saturating_add(u64::try_from(wait.as_millis()).unwrap_or(0));
                        sleep(wait);
                    }
                    _ => break,
                }
            }
        }
    }
    Err(last)
}

fn backoff_after(attempt: usize) -> Option<Duration> {
    RETRY_BACKOFF_MS
        .get(attempt)
        .map(|&ms| Duration::from_millis(ms))
}

pub fn clear_contents(root: &Path, counter: &Counter) -> Removal {
    let mut out = Removal::default();

    let Ok(entries) = std::fs::read_dir(root) else {
        out.failures.push(Failure {
            path: root.display().to_string(),
            reason: "cannot read folder".to_string(),
        });
        return out;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        remove_entry(&path, &mut out, counter);
    }
    out
}

pub fn remove_dir(root: &Path, counter: &Counter) -> Removal {
    let mut out = Removal::default();

    if fspath::is_reparse_path(root) {
        return out;
    }

    let walk = walkdir::WalkDir::new(root)
        .follow_links(false)
        .contents_first(true)
        .into_iter()
        .filter_entry(|e| !e.path_is_symlink() && !fspath::is_reparse_path(e.path()));

    for entry in walk {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        counter.tick();
        out.entries += 1;
        make_writable(path);
        let bytes = std::fs::symlink_metadata(path)
            .map(|m| m.len())
            .unwrap_or(0);
        let is_dir = entry.file_type().is_dir();
        let result = if is_dir {
            retry(&mut out, || std::fs::remove_dir(path))
        } else {
            retry(&mut out, || std::fs::remove_file(path))
        };
        match result {
            Ok(()) => {
                if !is_dir {
                    out.files_removed += 1;
                }
                out.freed_bytes = out.freed_bytes.saturating_add(bytes);
            }
            Err(err) => fail(&mut out, path, &err),
        }
    }
    out
}

fn remove_entry(path: &Path, out: &mut Removal, counter: &Counter) {
    let Ok(link_meta) = std::fs::symlink_metadata(path) else {
        return;
    };

    if fspath::is_reparse(&link_meta) {
        return;
    }

    counter.tick();
    out.entries += 1;

    if link_meta.is_dir() {
        out.merge(clear_contents(path, counter));
        match retry(out, || std::fs::remove_dir(path)) {
            Ok(()) => {}
            Err(err) => fail(out, path, &err),
        }
    } else {
        make_writable(path);
        let bytes = link_meta.len();
        match retry(out, || std::fs::remove_file(path)) {
            Ok(()) => {
                out.files_removed += 1;
                out.freed_bytes = out.freed_bytes.saturating_add(bytes);
            }
            Err(err) => fail(out, path, &err),
        }
    }
}

pub fn clean(path: &Path, remove_root: bool, counter: &Counter) -> Result<(Removal, bool)> {
    if !path.exists() {
        return Err(AppError::NotFound(path.display().to_string()));
    }

    let rehearsal = dry_run();
    if rehearsal {
        let size = super::size::dir_size(path);
        return Ok((
            Removal {
                freed_bytes: size.bytes,
                entries: size.files,
                ..Removal::default()
            },
            true,
        ));
    }

    let started = Instant::now();
    let should_remove_root = remove_root && !keep_root();
    let mut out = if should_remove_root {
        remove_dir(path, counter)
    } else if path.is_dir() {
        clear_contents(path, counter)
    } else {
        let mut out = Removal::default();
        remove_entry(path, &mut out, counter);
        out
    };
    out.elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(0);

    Ok((out, false))
}
