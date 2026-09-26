//! `.typdoc/lock.json` and the pinned copies under `.typdoc/vendor/schemas/` (SPC-16). Only
//! read: nothing here writes them or fetches.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::Report;
use crate::error::Error;

const LOCK_FILE: &str = ".typdoc/lock.json";
const VENDOR_SCHEMAS_DIR: &str = ".typdoc/vendor/schemas";

/// No `deny_unknown_fields`: an entry also holds `fetchedAt`, which nothing reads.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Pin {
    pub sha256: String,
}

/// Other sections may join `schemas` (SPC-16), so an unknown top-level key is left alone.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct Lock {
    #[serde(default)]
    pub schemas: BTreeMap<String, Pin>,
}

/// A missing file is no pins, so every remote schema is `config.schema-unpinned`. A file that
/// cannot be parsed has no config error id (SPC-6) and stops the command, as an unreadable
/// schema does.
pub(crate) fn read(root: &Path) -> Result<Lock, Error> {
    let file = root.join(LOCK_FILE);
    let bytes = match fs::read(&file) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Lock::default()),
        Err(source) => return Err(Error::Io { file, source }),
    };
    serde_json::from_slice(&bytes).map_err(|e| Error::Config {
        file,
        message: format!("cannot be parsed: {e}"),
    })
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The vendor path and text of the pinned copy of `url` (SPC-16), or `Ok(None)` once a config
/// error has been added to `report`. `said_in` is the file that named `url`, used as the
/// finding's `path`, as `config.schema-url` does.
pub(crate) fn read_pinned(
    root: &Path,
    url: &str,
    said_in: &str,
    report: &mut Report,
) -> Result<Option<(String, String)>, Error> {
    let lock = read(root)?;
    let Some(pin) = lock.schemas.get(url) else {
        report.add(
            "config.schema-unpinned",
            said_in,
            format!("the schema `{url}` has no pin: run `typdoc pull`"),
        );
        return Ok(None);
    };
    let vendor_path = format!("{VENDOR_SCHEMAS_DIR}/{}", pin.sha256);
    let file = root.join(&vendor_path);
    let bytes = match fs::read(&file) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            report.add(
                "config.vendor-missing",
                said_in,
                format!(
                    "the pinned copy of `{url}` is missing at `{vendor_path}`: run `typdoc pull`"
                ),
            );
            return Ok(None);
        }
        Err(source) => return Err(Error::Io { file, source }),
    };
    let actual = sha256_hex(&bytes);
    if actual != pin.sha256 {
        report.add(
            "config.vendor-edited",
            said_in,
            format!(
                "the pinned copy of `{url}` at `{vendor_path}` was edited by hand (its contents hash to `{actual}`, not `{}`): run `typdoc pull`",
                pin.sha256
            ),
        );
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Ok(Some((vendor_path, text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NIST's published vectors (FIPS 180-4), not this crate's own output.
    #[test]
    fn sha256_hex_matches_the_published_test_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
