//! Manifests on a disk: what counts as one, and how to read and write it
//! whether it is packed or not (ADR-0023).
//!
//! A manifest is JSON, and a directory of them is what the application loads.
//! Since 2026-09-10 a manifest may be stored gzipped, `<name>.json.gz`, which
//! takes the issued library from about 311 MB to about 20 MB — the same
//! records, a twentieth of the bytes. Both forms load; the packing is not
//! part of what a manifest says.

use std::io::{Read, Write};
use std::path::Path;

/// What a plain manifest is called.
pub const MANIFEST_SUFFIX: &str = ".json";
/// What a packed one is called.
pub const COMPRESSED_SUFFIX: &str = ".json.gz";

/// Whether a file name in a library directory is a manifest to load.
///
/// A name beginning with a dot never is: macOS writes an AppleDouble sidecar
/// — `._platform.json` beside `platform.json` — whenever a folder crosses a
/// stick or a share, invisible in Finder and ending in `.json` all the same.
/// Counting one as a manifest refuses a copy that has not lost a byte.
pub fn is_manifest(name: &str) -> bool {
    if name.starts_with('.') {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    lower.ends_with(MANIFEST_SUFFIX) || lower.ends_with(COMPRESSED_SUFFIX)
}

/// Whether this name is stored packed.
pub fn is_compressed(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(COMPRESSED_SUFFIX)
}

/// The name a manifest would have if it were packed, and if it already is,
/// itself.
pub fn compressed_name(name: &str) -> String {
    if is_compressed(name) {
        return name.to_string();
    }
    match name.strip_suffix(MANIFEST_SUFFIX) {
        Some(stem) => format!("{stem}{COMPRESSED_SUFFIX}"),
        None => format!("{name}{COMPRESSED_SUFFIX}"),
    }
}

/// Read one manifest as JSON text, decompressing it when it is packed.
pub fn read(path: &Path) -> std::io::Result<String> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !is_compressed(name) {
        return std::fs::read_to_string(path);
    }
    let packed = std::fs::read(path)?;
    decompress(&packed)
}

/// Write one manifest, packing it when the name says so.
pub fn write(path: &Path, text: &str) -> std::io::Result<()> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !is_compressed(name) {
        return std::fs::write(path, text);
    }
    std::fs::write(path, compress(text)?)
}

/// JSON text as packed bytes.
pub fn compress(text: &str) -> std::io::Result<Vec<u8>> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(text.as_bytes())?;
    encoder.finish()
}

/// Packed bytes as JSON text.
pub fn decompress(packed: &[u8]) -> std::io::Result<String> {
    let mut text = String::new();
    flate2::read::GzDecoder::new(packed).read_to_string(&mut text)?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_is_json_packed_or_not_and_never_a_sidecar() {
        assert!(is_manifest("platform.json"));
        assert!(is_manifest("dtc_help.json.gz"));
        assert!(is_manifest("PLATFORM.JSON"));
        // The stamp is a manifest by name and not by nature; the loader
        // excludes it by name, which is its own decision, not this one.
        assert!(is_manifest("issued_to.json"));
        assert!(!is_manifest("._platform.json"));
        assert!(!is_manifest("._dtc_help.json.gz"));
        assert!(!is_manifest("notes.txt"));
        assert!(!is_manifest("platform.json.bak"));
    }

    #[test]
    fn packing_a_manifest_and_reading_it_back_returns_what_went_in() {
        let text = r#"{"schema_version":1,"records":[{"id":"a"},{"id":"b"}]}"#;
        let packed = compress(text).unwrap();
        assert!(
            packed.len() < text.len() + 64,
            "gzip adds a header; it must not double the file"
        );
        assert_eq!(decompress(&packed).unwrap(), text);
    }

    #[test]
    fn a_name_that_is_already_packed_is_left_alone() {
        assert_eq!(compressed_name("platform.json"), "platform.json.gz");
        assert_eq!(compressed_name("platform.json.gz"), "platform.json.gz");
        assert!(is_compressed("platform.json.gz"));
        assert!(!is_compressed("platform.json"));
    }

    #[test]
    fn what_is_not_gzip_is_a_failure_and_not_an_empty_manifest() {
        assert!(decompress(b"{\"schema_version\":1}").is_err());
    }
}
