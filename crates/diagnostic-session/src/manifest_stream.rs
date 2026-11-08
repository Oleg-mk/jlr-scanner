//! A library directory read one manifest at a time (`ADR-0040`).
//!
//! The loader reads each file twice and keeps neither reading. The first
//! pass hashes the decompressed text as it streams past — the stamp is
//! checked against those hashes — keeps the first few kilobytes, where an
//! issue code would be found, and checks that the whole is well-formed
//! JSON. The second pass hands each manifest's own text to the ingest as it
//! is read, one at a time, and hashes the bytes again, so that a file
//! changed between the two passes is caught. Until 2026-11-08 every text,
//! a generic JSON tree of each bundle and a second parse of every manifest
//! were resident together, and loading the full library peaked at 5 GB.

use serde::de::{IgnoredAny, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer as _};
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use std::fmt;
use std::io::{self, BufRead, BufReader, Read};

/// How much of a file's start is kept for the issue code; `issue` looks no
/// further either.
pub const HEAD_BYTES: usize = 16_384;

/// The bytes going by, hashed, with the first of them kept.
struct Tee<R> {
    inner: R,
    hasher: Sha256,
    head: Vec<u8>,
}

impl<R> Tee<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
            head: Vec::new(),
        }
    }

    fn finish(self) -> (String, Vec<u8>) {
        (format!("{:x}", self.hasher.finalize()), self.head)
    }
}

impl<R: Read> Read for Tee<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        self.hasher.update(&buffer[..read]);
        let room = HEAD_BYTES.saturating_sub(self.head.len()).min(read);
        self.head.extend_from_slice(&buffer[..room]);
        Ok(read)
    }
}

/// What the first pass learns of one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scan {
    /// SHA-256 of the decompressed text, lowercase hex, as a stamp names it.
    pub sha256: String,
    /// The start of the text, where an issue code is looked for.
    pub head: String,
    /// Why the text is not well-formed JSON, when it is not.
    pub json_error: Option<String>,
}

/// Hash one file's text as it streams past and check that it is JSON. A
/// failure to read it — a packed file that is not gzip — is the caller's to
/// record; a text that is not JSON is reported in the scan, and its every
/// byte is hashed all the same.
pub fn scan(reader: impl Read) -> io::Result<Scan> {
    let mut buffered = BufReader::new(Tee::new(reader));
    let json_error = {
        let mut deserializer = serde_json::Deserializer::from_reader(&mut buffered);
        let checked = IgnoredAny::deserialize(&mut deserializer).and_then(|_| deserializer.end());
        match checked {
            Ok(()) => None,
            Err(error) if error.is_io() => return Err(io::Error::other(error)),
            Err(error) => Some(error.to_string()),
        }
    };
    // Whatever the parser left unread is hashed too.
    io::copy(&mut buffered, &mut io::sink())?;
    let (sha256, head) = buffered.into_inner().finish();
    Ok(Scan {
        sha256,
        head: String::from_utf8_lossy(&head).into_owned(),
        json_error,
    })
}

/// Each manifest of one file as its own JSON text, in order — every element
/// of a bundle, one at a time, or the whole of a single manifest — and then
/// how many there were and the SHA-256 of every byte read.
pub fn for_each_manifest(
    reader: impl Read,
    mut each: impl FnMut(usize, &str),
) -> Result<(usize, String), String> {
    let mut buffered = BufReader::new(Tee::new(reader));
    let first = first_significant_byte(&mut buffered).map_err(|error| error.to_string())?;
    let count = {
        let mut deserializer = serde_json::Deserializer::from_reader(&mut buffered);
        let count = if first == Some(b'[') {
            (&mut deserializer)
                .deserialize_seq(Elements(&mut each))
                .map_err(|error| error.to_string())?
        } else {
            let whole = Box::<RawValue>::deserialize(&mut deserializer)
                .map_err(|error| error.to_string())?;
            each(0, whole.get());
            1
        };
        deserializer.end().map_err(|error| error.to_string())?;
        count
    };
    io::copy(&mut buffered, &mut io::sink()).map_err(|error| error.to_string())?;
    let (sha256, _) = buffered.into_inner().finish();
    Ok((count, sha256))
}

/// The first byte that is not JSON whitespace, left unread.
fn first_significant_byte(reader: &mut impl BufRead) -> io::Result<Option<u8>> {
    loop {
        let buffer = reader.fill_buf()?;
        let Some(&byte) = buffer.first() else {
            return Ok(None);
        };
        if matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
            reader.consume(1);
            continue;
        }
        return Ok(Some(byte));
    }
}

/// The elements of a bundle, each handed over as its own text and dropped
/// before the next is read.
struct Elements<'a, F>(&'a mut F);

impl<'de, F: FnMut(usize, &str)> Visitor<'de> for Elements<'_, F> {
    type Value = usize;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an array of manifests")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<usize, A::Error> {
        let mut count = 0;
        while let Some(manifest) = sequence.next_element::<Box<RawValue>>()? {
            (self.0)(count, manifest.get());
            count += 1;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha256(text: &str) -> String {
        format!("{:x}", Sha256::digest(text.as_bytes()))
    }

    #[test]
    fn a_scan_hashes_every_byte_and_keeps_only_the_head() {
        let text = format!("[{}]", vec![r#"{"a":1}"#; 5000].join(","));
        let scan = scan(text.as_bytes()).unwrap();
        assert_eq!(scan.sha256, sha256(&text));
        assert_eq!(scan.head.len(), HEAD_BYTES);
        assert!(text.starts_with(&scan.head));
        assert_eq!(scan.json_error, None);
    }

    #[test]
    fn a_text_that_is_not_json_is_reported_and_still_hashed_whole() {
        let text = r#"[{"a":1}, {"b": ]   trailing bytes that come after the error"#;
        let scan = scan(text.as_bytes()).unwrap();
        assert!(scan.json_error.is_some());
        assert_eq!(scan.sha256, sha256(text));
        // Trailing content after a well-formed value is not JSON either.
        assert!(super::scan(&b"{} {}"[..]).unwrap().json_error.is_some());
    }

    #[test]
    fn a_reader_that_fails_is_an_error_not_a_scan() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid gzip header",
                ))
            }
        }
        assert!(scan(Broken).is_err());
    }

    #[test]
    fn a_bundle_is_handed_over_one_manifest_at_a_time_in_order() {
        let text =
            "  \n[ {\"id\": \"one\"}, {\"id\": \"two\", \"x\": [1, 2]} , {\"id\":\"three\"} ]\n";
        let mut seen = Vec::new();
        let (count, hash) = for_each_manifest(text.as_bytes(), |index, manifest| {
            seen.push((index, manifest.to_string()))
        })
        .unwrap();
        assert_eq!(count, 3);
        assert_eq!(hash, sha256(text));
        assert_eq!(
            seen,
            vec![
                (0, r#"{"id": "one"}"#.to_string()),
                (1, r#"{"id": "two", "x": [1, 2]}"#.to_string()),
                (2, r#"{"id":"three"}"#.to_string()),
            ]
        );
    }

    #[test]
    fn a_single_manifest_is_handed_over_whole_and_an_empty_bundle_hands_over_nothing() {
        let single = r#"{"schema_version": 1, "records": []}"#;
        let mut seen = Vec::new();
        let (count, _) = for_each_manifest(single.as_bytes(), |_, manifest| {
            seen.push(manifest.to_string())
        })
        .unwrap();
        assert_eq!((count, seen), (1, vec![single.to_string()]));

        let (count, _) =
            for_each_manifest(&b"[]"[..], |_, _| panic!("nothing to hand over")).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn a_bundle_that_breaks_off_is_an_error() {
        let text = r#"[{"id":"one"}, {"id":"#;
        assert!(for_each_manifest(text.as_bytes(), |_, _| {}).is_err());
    }
}
