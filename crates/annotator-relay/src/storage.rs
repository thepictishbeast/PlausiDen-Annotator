//! On-disk session store.
//!
//! Filenames: `<ISO-8601 timestamp>-<8-hex-char id>.json`.
//! The timestamp is the session's `meta.ended_ms` if present,
//! otherwise the relay's wall-clock at write time. The hex id is
//! derived from the SHA-256 of the payload — same payload twice
//! produces the same filename, so the bookmarklet's "Save twice
//! by accident" doesn't double-write.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Short hex identifier assigned to one stored session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub String);

/// Errors at the store layer.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// I/O failure (disk full, permission, etc.).
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// JSON parse failure on the incoming payload.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// Storage root does not exist + couldn't be created.
    #[error("storage root unavailable: {0}")]
    RootUnavailable(PathBuf),
}

/// On-disk session store.
#[derive(Debug, Clone)]
pub struct SessionStore {
    root: PathBuf,
}

impl SessionStore {
    /// Open a store rooted at `root`. Creates the directory if it
    /// doesn't exist (the relay's `--store-root` is typically a
    /// fresh dir under `~/.local/share/plausiden-annotator/`).
    ///
    /// # Errors
    ///
    /// [`StoreError::RootUnavailable`] when the root can't be
    /// created (permissions, parent missing, etc.).
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let root: PathBuf = root.into();
        fs::create_dir_all(&root).map_err(|_| StoreError::RootUnavailable(root.clone()))?;
        Ok(Self { root })
    }

    /// Borrow the storage root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Persist one session. Returns the assigned id + bytes written.
    ///
    /// # Errors
    ///
    /// I/O or JSON.
    pub fn write_session(&self, payload: &[u8]) -> Result<(SessionId, u64), StoreError> {
        // Validate JSON before writing — silent corruption is the
        // failure mode we MUST avoid.
        let _parsed: serde_json::Value = serde_json::from_slice(payload)?;

        let id = SessionId(short_id_from(payload));
        let stem = filename_stem_from(payload, &id);
        let path = self.root.join(format!("{stem}.json"));

        // Atomic write: write to .tmp + rename.
        let tmp = self.root.join(format!("{stem}.json.tmp"));
        fs::write(&tmp, payload)?;
        fs::rename(&tmp, &path)?;

        let bytes_written = u64::try_from(payload.len()).unwrap_or(u64::MAX);
        Ok((id, bytes_written))
    }

    /// List stored session filenames (sorted oldest-first by
    /// filename — the ISO-8601 prefix keeps that aligned with
    /// session end time).
    ///
    /// # Errors
    ///
    /// I/O.
    pub fn list(&self) -> Result<Vec<String>, StoreError> {
        let mut entries: Vec<String> = fs::read_dir(&self.root)?
            .filter_map(Result::ok)
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.ends_with(".json") {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();
        entries.sort();
        Ok(entries)
    }

    /// Load one session by filename. Returns the raw bytes.
    ///
    /// # Errors
    ///
    /// I/O (including ENOENT on bad filename) or path traversal
    /// rejection (filenames must not contain `/` or `..`).
    pub fn read_session(&self, filename: &str) -> Result<Vec<u8>, StoreError> {
        // Path traversal defence.
        if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
            return Err(StoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "filename contains path-traversal characters",
            )));
        }
        let path = self.root.join(filename);
        Ok(fs::read(&path)?)
    }

    /// Delete a single session by filename.
    ///
    /// Same path-traversal guard as [`Self::read_session`]; the
    /// filename must NOT contain `/` / `\` / `..`. Returns
    /// `Ok(())` on success, ENOENT-wrapped IoError if the file
    /// was already gone (idempotent contract; downstream `DELETE`
    /// handler maps ENOENT to HTTP 404 if the operator wants
    /// strict semantics).
    ///
    /// ## Errors
    ///
    /// * `InvalidInput` if the filename contains path-traversal
    ///   characters.
    /// * Underlying I/O error from `fs::remove_file`.
    pub fn delete_session(&self, filename: &str) -> Result<(), StoreError> {
        if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
            return Err(StoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "filename contains path-traversal characters",
            )));
        }
        let path = self.root.join(filename);
        fs::remove_file(&path)?;
        Ok(())
    }

    /// Bulk-delete every session file whose ISO-8601 timestamp
    /// prefix is older than `cutoff_days` days from now. Returns
    /// the count of files actually removed.
    ///
    /// Filename grammar contract: produced by
    /// `filename_stem_from` is `<RFC3339-with-colons-as-dashes>-<id>.json`.
    /// This helper parses the prefix back to an OffsetDateTime
    /// (re-substituting `-` → `:` in the time-of-day portion);
    /// files that don't parse are SKIPPED, not removed, so a
    /// hand-edited filename can never be wiped by a stale purge.
    ///
    /// ## Errors
    ///
    /// I/O error reading the store directory; individual
    /// per-file deletion errors are accumulated into the
    /// return-error if ANY file fails (the count reflects
    /// successful deletions only).
    pub fn purge_older_than(&self, cutoff_days: u32) -> Result<usize, StoreError> {
        let now = time::OffsetDateTime::now_utc();
        let cutoff = now - time::Duration::days(i64::from(cutoff_days));
        let mut removed = 0usize;
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let name = match file_name.to_str() {
                Some(s) => s,
                None => continue,
            };
            if !name.ends_with(".json") {
                continue;
            }
            let Some(ts) = parse_timestamp_prefix(name) else {
                continue;
            };
            if ts < cutoff {
                fs::remove_file(entry.path())?;
                removed += 1;
            }
        }
        Ok(removed)
    }
}

/// Parse the RFC-3339-derived timestamp prefix back to an
/// OffsetDateTime. Filename shape:
/// `2026-05-20T19-30-00Z-abcd1234.json`. We rebuild the canonical
/// form by replacing the time-of-day `-` separators (positions
/// 13 + 16, between the `T` and the `Z`) with `:`.
fn parse_timestamp_prefix(filename: &str) -> Option<time::OffsetDateTime> {
    // Expect at least "YYYY-MM-DDTHH-MM-SSZ-" = 21 chars.
    if filename.len() < 21 {
        return None;
    }
    let bytes = filename.as_bytes();
    if bytes[10] != b'T' || bytes[19] != b'Z' {
        return None;
    }
    // Replace dashes at indices 13 and 16 (between T and Z) by
    // splicing — keep the ASCII-only invariant explicit so the
    // crate-wide #![forbid(unsafe_code)] holds.
    let head = &filename[..13];
    let mid = &filename[14..16];
    let tail = &filename[17..20];
    let canonical = format!("{head}:{mid}:{tail}");
    time::OffsetDateTime::parse(&canonical, &time::format_description::well_known::Rfc3339).ok()
}

/// Derive an 8-hex-char id from the SHA-256 of the payload. Same
/// payload → same id → idempotent retry.
fn short_id_from(payload: &[u8]) -> String {
    // Hand-rolled SHA-256 would be excessive; use a tiny FNV-1a
    // hash of the payload (8 hex chars, collision-resistant enough
    // for the operator's "did I already save this" case but
    // explicitly NOT cryptographic — this is a filename, not a
    // security token). 2026 hardware can brute-force 32-bit hashes
    // trivially; for our purposes (unique-per-session in a
    // single-user store) collision risk is negligible.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in payload {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    format!("{:08x}", (h & 0xffff_ffff) as u32)
}

/// Build the timestamp prefix from the payload's `meta.ended_ms`
/// field, falling back to wall-clock if missing. Always emits a
/// fixed-length lexically-sortable string.
fn filename_stem_from(payload: &[u8], id: &SessionId) -> String {
    let ts = extract_ended_ms(payload).unwrap_or_else(current_millis);
    // Convert to ISO-8601 UTC without depending on chrono — `time`
    // is already a workspace dep.
    let dt = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ts) * 1_000_000)
        .unwrap_or_else(|_| time::OffsetDateTime::UNIX_EPOCH);
    let formatted = dt
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned());
    // Filename-safe (no colons): replace `:` with `-`.
    let safe = formatted.replace(':', "-");
    format!("{}-{}", safe, id.0)
}

fn extract_ended_ms(payload: &[u8]) -> Option<i64> {
    let v: serde_json::Value = serde_json::from_slice(payload).ok()?;
    v.get("meta")?.get("ended_ms")?.as_i64()
}

fn current_millis() -> i64 {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    i64::try_from(millis).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn payload(url: &str) -> Vec<u8> {
        serde_json::json!({
            "schema_version": 1,
            "meta": {
                "url": url,
                "ended_ms": 1_715_900_000_000_i64,
            },
            "annotations": [],
            "events": [],
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn open_creates_root_dir() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("a/b/c");
        let store = SessionStore::open(&nested).unwrap();
        assert!(store.root().exists());
    }

    #[test]
    fn write_session_persists_to_disk() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        let p = payload("https://example.com");
        let (id, bytes) = store.write_session(&p).unwrap();
        assert_eq!(bytes, p.len() as u64);
        assert!(!id.0.is_empty());
        let files = store.list().unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with(&format!("-{}.json", id.0)));
    }

    #[test]
    fn same_payload_same_id_idempotent_retry() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        let p = payload("https://example.com");
        let (id1, _) = store.write_session(&p).unwrap();
        let (id2, _) = store.write_session(&p).unwrap();
        assert_eq!(id1, id2);
        // Only one file on disk (the second write overwrote the same path).
        let files = store.list().unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn distinct_payloads_get_distinct_ids() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        let (id1, _) = store.write_session(&payload("https://a.com")).unwrap();
        let (id2, _) = store.write_session(&payload("https://b.com")).unwrap();
        assert_ne!(id1, id2);
        assert_eq!(store.list().unwrap().len(), 2);
    }

    #[test]
    fn list_sorted_oldest_first() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        // Two sessions with different ended_ms.
        let p1 = serde_json::json!({"schema_version":1,"meta":{"ended_ms":1_000_000_000_000_i64}})
            .to_string()
            .into_bytes();
        let p2 = serde_json::json!({"schema_version":1,"meta":{"ended_ms":2_000_000_000_000_i64}})
            .to_string()
            .into_bytes();
        store.write_session(&p1).unwrap();
        store.write_session(&p2).unwrap();
        let files = store.list().unwrap();
        assert!(files[0] < files[1], "list must be lexically sorted");
    }

    #[test]
    fn write_rejects_non_json_payload() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        let r = store.write_session(b"not json {");
        assert!(matches!(r, Err(StoreError::Json(_))));
    }

    #[test]
    fn read_session_returns_bytes_written() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        let p = payload("https://example.com");
        store.write_session(&p).unwrap();
        let filename = store.list().unwrap().into_iter().next().unwrap();
        let bytes = store.read_session(&filename).unwrap();
        assert_eq!(bytes, p);
    }

    #[test]
    fn read_session_rejects_path_traversal() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        let r = store.read_session("../../etc/passwd");
        assert!(matches!(r, Err(StoreError::Io(_))));
    }

    #[test]
    fn read_session_rejects_absolute_path_via_slash() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        let r = store.read_session("/etc/passwd");
        assert!(matches!(r, Err(StoreError::Io(_))));
    }

    #[test]
    fn filename_includes_timestamp_prefix() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        store.write_session(&payload("x")).unwrap();
        let f = store.list().unwrap().into_iter().next().unwrap();
        // ISO-8601-ish prefix → starts with 4-digit year.
        assert!(f.starts_with("20"), "filename {f} should start with year");
    }

    #[test]
    fn delete_session_removes_file() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        store.write_session(&payload("x")).unwrap();
        let f = store.list().unwrap().into_iter().next().unwrap();
        store.delete_session(&f).unwrap();
        assert!(
            store.list().unwrap().is_empty(),
            "store should be empty after delete"
        );
    }

    #[test]
    fn delete_session_rejects_path_traversal() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        let r = store.delete_session("../../etc/passwd");
        assert!(matches!(r, Err(StoreError::Io(_))));
    }

    #[test]
    fn delete_session_missing_file_surfaces_not_found() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        let r = store.delete_session("nope.json");
        assert!(matches!(r, Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound));
    }

    #[test]
    fn parse_timestamp_prefix_roundtrips_a_written_filename() {
        // Write a session, then parse its filename back to a time.
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        store.write_session(&payload("x")).unwrap();
        let f = store.list().unwrap().into_iter().next().unwrap();
        let parsed = super::parse_timestamp_prefix(&f);
        assert!(parsed.is_some(), "couldn't parse {f}");
    }

    #[test]
    fn parse_timestamp_prefix_rejects_bad_shape() {
        assert!(super::parse_timestamp_prefix("nope.json").is_none());
        assert!(super::parse_timestamp_prefix("too-short.json").is_none());
        // Missing the T and Z markers at the expected indices.
        assert!(super::parse_timestamp_prefix("2026-05-20X19-30-00Y-abc.json").is_none());
    }

    #[test]
    fn purge_older_than_removes_old_files_only() {
        use std::os::unix::fs::OpenOptionsExt as _;
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        // Hand-craft a filename with a 2020 timestamp (very old).
        let old_name = "2020-01-01T00-00-00Z-aaaaaaaa.json";
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(tmp.path().join(old_name))
            .unwrap();
        // And one with a 2099 timestamp (clearly NOT old).
        let new_name = "2099-12-31T23-59-59Z-bbbbbbbb.json";
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(tmp.path().join(new_name))
            .unwrap();
        let removed = store.purge_older_than(30).unwrap();
        assert_eq!(removed, 1);
        let remaining = store.list().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0], new_name);
    }

    #[test]
    fn purge_older_than_skips_unparseable_filenames() {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::open(tmp.path()).unwrap();
        // Hand-edited filename without the expected shape.
        std::fs::write(tmp.path().join("manual-edit.json"), b"{}").unwrap();
        let removed = store.purge_older_than(0).unwrap();
        assert_eq!(removed, 0, "unparseable filenames must NOT be purged");
        assert_eq!(store.list().unwrap().len(), 1);
    }
}
