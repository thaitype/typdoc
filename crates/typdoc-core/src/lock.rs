//! `.typdoc/lock.json`: the pin of every remote schema this project has fetched (`typdoc pull`,
//! story 3). Read only in this story: the file is never written, and nothing is fetched
//! (contract, decision 3 and 5).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::Report;
use crate::error::Error;

const LOCK_FILE: &str = ".typdoc/lock.json";
const VENDOR_SCHEMAS_DIR: &str = ".typdoc/vendor/schemas";

/// One entry of `lock.json`'s `schemas` object. `fetchedAt` is read by nothing this story
/// needs; `serde` leaves it where it is, since `Pin` has no `deny_unknown_fields`.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Pin {
    pub sha256: String,
}

/// `lock.json` as this story reads it: only the `schemas` section exists yet (`pull` is story
/// 3; a later section joins it with no rename, the design says, so nothing here assumes it is
/// the only one — an unknown top-level key is left alone rather than refused, since this story
/// does not own the file's shape the way it owns `config.json`'s).
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct Lock {
    #[serde(default)]
    pub schemas: BTreeMap<String, Pin>,
}

/// `.typdoc/lock.json`, or an empty lock (no pins at all) when the file does not exist: a
/// project that has never run `pull` has no `lock.json` yet, and every remote schema it names is
/// `config.schema-unpinned` either way. A file that exists and cannot be read as the shape above
/// has no id in the design's table (the file is written and re-read by `pull`, never hand-edited
/// under the design's own account), so it stops the command the same id-less way an unreadable
/// schema already does.
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

/// The SHA-256 of `bytes`, as lowercase hex: what a pinned copy's contents are checked against
/// its own file name with (`config.vendor-edited`).
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The text of the pinned copy of the remote schema named by `url`, checked against
/// `lock.json`, or `Ok(None)` once a config error (`config.schema-unpinned`,
/// `config.vendor-missing` or `config.vendor-edited`) has been added to `report` and the caller
/// stops reading this chain, the same way an unreadable local schema already does
/// (`schema::load`). `said_in` is the file the reference was named in, for the message and as
/// the finding's `path` — the same choice `config.schema-url` already makes for a URL of the
/// wrong scheme. `url` is used unchanged as the lookup key in `lock.json`'s `schemas` object
/// (contract, decision 3: "a pinned copy is read from `vendor/schemas/<sha256>`... an absent
/// copy is `config.vendor-missing`... a URL with no pin is `config.schema-unpinned`"), and, once
/// found, as `vendor/schemas/<sha256>`'s own file name to check the copy's bytes against
/// (`config.vendor-edited`).
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

    /// NIST's own published test vectors (FIPS 180-4), not the output of this crate or of
    /// `typdoc pull`: proof the hash this check relies on is computed right, independent of
    /// anything this story writes.
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
