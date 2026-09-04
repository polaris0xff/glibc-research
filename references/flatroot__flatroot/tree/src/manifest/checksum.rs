//! `Checksum`: an algorithm paired with its digest, with the
//! `algorithm:digest` text form the manifest stores and the
//! verification that accepts or rejects a downloaded archive.

use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result, bail};

/// The closed set of checksum algorithms the supported distros use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
  /// Lowercase hexadecimal SHA-256 of the whole archive — the
  /// common case across Debian, Ubuntu, Arch, the RPM family,
  /// and CachyOS.
  Sha256,
  /// Lowercase hexadecimal SHA-512 of the whole archive, used
  /// by openSUSE.
  Sha512,
  /// Alpine's `Q1`-prefixed base64 SHA-1 of only the second
  /// gzip member's compressed bytes inside the multi-stream
  /// `.apk` (signature / control / data). Distinct from any
  /// full-file SHA-1.
  Q1Sha1,
}

impl ChecksumAlgorithm {
  /// The algorithm's stable short name for the on-disk manifest, so a
  /// round trip recovers it unchanged.
  fn as_str(self) -> &'static str {
    match self {
      ChecksumAlgorithm::Sha256 => "sha256",
      ChecksumAlgorithm::Sha512 => "sha512",
      ChecksumAlgorithm::Q1Sha1 => "q1-sha1",
    }
  }

  /// Recovers the algorithm from the digest's own form — the `Q1`
  /// marker or the digest length — since a distro index states the
  /// value but never the algorithm.
  pub fn infer(digest: &str) -> Self {
    if digest.starts_with("Q1") {
      ChecksumAlgorithm::Q1Sha1
    } else if digest.len() == 128 {
      ChecksumAlgorithm::Sha512
    } else {
      ChecksumAlgorithm::Sha256
    }
  }
}

impl FromStr for ChecksumAlgorithm {
  type Err = anyhow::Error;

  /// Parses an algorithm name back from the manifest; an unrecognized
  /// name is a corrupt record, not a guess.
  fn from_str(s: &str) -> Result<Self> {
    match s {
      "sha256" => Ok(ChecksumAlgorithm::Sha256),
      "sha512" => Ok(ChecksumAlgorithm::Sha512),
      "q1-sha1" => Ok(ChecksumAlgorithm::Q1Sha1),
      other => bail!("unknown checksum algorithm '{}'", other),
    }
  }
}

/// One archive's checksum: the algorithm and digest kept together so
/// they never drift apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checksum {
  /// Which scheme produced the digest held alongside it.
  pub algorithm: ChecksumAlgorithm,
  /// The encoded digest: lowercase hex for SHA-256/512, or Alpine's
  /// verbatim `Q1...` base64 with its marker.
  pub hex: String,
}

impl Checksum {
  /// Renders the checksum as one `algorithm:digest` line for the
  /// manifest.
  pub fn format(&self) -> String {
    format!("{}:{}", self.algorithm.as_str(), self.hex)
  }

  /// Parses a stored `algorithm:digest` line; a line missing the `:` is
  /// a corrupt record.
  pub fn parse(s: &str) -> Result<Self> {
    let (algo, hex) = s
      .split_once(':')
      .with_context(|| format!("checksum '{s}' missing ':'"))?;
    Ok(Checksum {
      algorithm: algo.parse()?,
      hex: hex.to_string(),
    })
  }

  /// Builds a `Checksum` from a bare index digest by inferring its
  /// algorithm — the fresh-download path, as opposed to `parse`, which
  /// reads a stored line back.
  pub fn infer(digest: &str) -> Self {
    Checksum {
      algorithm: ChecksumAlgorithm::infer(digest),
      hex: digest.to_string(),
    }
  }

  /// Whether the archive at `path` hashes to this digest — the check
  /// that trusts a cached copy or accepts a download. Alpine's digest
  /// covers only one inner gzip stream, so those bytes are extracted
  /// before hashing.
  pub fn verify(&self, path: &Path) -> Result<bool> {
    use sha2::Digest;

    let bytes = std::fs::read(path)?;
    match self.algorithm {
      ChecksumAlgorithm::Q1Sha1 => Self::q1_sha1_verify(&bytes, &self.hex),
      ChecksumAlgorithm::Sha512 => Ok(hex::encode(sha2::Sha512::digest(&bytes)) == self.hex),
      ChecksumAlgorithm::Sha256 => Ok(hex::encode(sha2::Sha256::digest(&bytes)) == self.hex),
    }
  }

  /// Alpine's digest covers the second gzip member's compressed bytes —
  /// the control stream of the signature/control/data sequence — not
  /// the whole file, so that member is extracted before hashing.
  fn q1_sha1_verify(bytes: &[u8], digest: &str) -> Result<bool> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use sha2::Digest;

    let b64 = digest.strip_prefix("Q1").unwrap_or(digest);
    let control_bytes = Self::gzip_member_second(bytes);

    let hash = sha1::Sha1::digest(control_bytes);
    let expected_bytes = STANDARD
      .decode(b64)
      .with_context(|| format!("apk checksum 'Q1{b64}' is not valid base64"))?;
    Ok(hash[..] == expected_bytes[..])
  }

  /// The slice holding the second gzip member's compressed bytes.
  /// Boundaries are found by decompressing each member and reading how
  /// far the cursor advanced — bufread::GzDecoder (not read::GzDecoder)
  /// avoids an internal BufReader that reads ahead and would make the
  /// position inaccurate. Data without recognizable members falls back
  /// to the whole file, which simply fails the later comparison rather
  /// than panicking.
  fn gzip_member_second(bytes: &[u8]) -> &[u8] {
    let mut stream_boundaries = vec![0usize];
    let mut pos = 0;
    while pos + 2 <= bytes.len() && bytes[pos] == 0x1f && bytes[pos + 1] == 0x8b {
      let cursor = std::io::Cursor::new(&bytes[pos..]);
      let mut decoder = flate2::bufread::GzDecoder::new(cursor);
      let mut buf = Vec::new();
      if std::io::Read::read_to_end(&mut decoder, &mut buf).is_err() {
        break;
      }
      let member_size = decoder.into_inner().position() as usize;
      pos += member_size;
      stream_boundaries.push(pos);
    }

    if stream_boundaries.len() >= 3 {
      return &bytes[stream_boundaries[1]..stream_boundaries[2]];
    }
    if stream_boundaries.len() >= 2 {
      return &bytes[stream_boundaries[1]..];
    }
    bytes
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn checksum_all_three_algorithms_roundtrip() {
    for algo in [
      ChecksumAlgorithm::Sha256,
      ChecksumAlgorithm::Sha512,
      ChecksumAlgorithm::Q1Sha1,
    ] {
      let c = Checksum {
        algorithm: algo,
        hex: "deadbeef".to_string(),
      };
      let encoded = c.format();
      assert_eq!(Checksum::parse(&encoded).unwrap(), c);
    }
    assert!(Checksum::parse("sha1:abc").is_err());
  }

  #[test]
  fn checksum_algorithm_infer_dispatches_on_shape() {
    assert_eq!(ChecksumAlgorithm::infer("Q1abcdef"), ChecksumAlgorithm::Q1Sha1);
    assert_eq!(ChecksumAlgorithm::infer(&"a".repeat(128)), ChecksumAlgorithm::Sha512);
    assert_eq!(ChecksumAlgorithm::infer(&"a".repeat(64)), ChecksumAlgorithm::Sha256);
    assert_eq!(ChecksumAlgorithm::infer("deadbeef"), ChecksumAlgorithm::Sha256);
  }
}
