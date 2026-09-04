//! `Codec`: detects a stream's compression format and decompresses it,
//! for the handful of formats distro package indexes and archives ship
//! in.

use std::io::{BufRead, BufReader, Read};

use anyhow::Result;

/// A stream's compression format, including `None` so an uncompressed
/// stream goes through the same path as a compressed one.
pub enum Codec {
  Gzip,
  Xz,
  Zstd,
  Bzip2,
  None,
}

impl Codec {
  /// The compression format named by a filename's suffix; an
  /// unrecognized suffix means the stream is uncompressed.
  pub fn from_suffix(name: &str) -> Self {
    if name.ends_with(".gz") {
      Codec::Gzip
    } else if name.ends_with(".xz") {
      Codec::Xz
    } else if name.ends_with(".zst") {
      Codec::Zstd
    } else if name.ends_with(".bz2") {
      Codec::Bzip2
    } else {
      Codec::None
    }
  }

  /// Sniffs the leading magic bytes to tell zstd from gzip, for archives
  /// whose name gives no hint and that ship in only those two forms.
  pub fn from_magic(bytes: &[u8]) -> Result<Self> {
    let is_zstd = bytes.len() >= 4 && bytes[0..4] == [0x28, 0xb5, 0x2f, 0xfd];
    if is_zstd { Ok(Codec::Zstd) } else { Ok(Codec::Gzip) }
  }

  /// Decompresses lazily into a reader, producing only as much as is
  /// consumed — for scanning a large index line by line without holding
  /// the whole thing in memory.
  pub fn reader<'a>(self, bytes: &'a [u8]) -> Result<Box<dyn BufRead + 'a>> {
    Ok(match self {
      Codec::Gzip => Box::new(BufReader::new(flate2::read::GzDecoder::new(bytes))),
      Codec::Xz => Box::new(BufReader::new(xz2::read::XzDecoder::new(bytes))),
      Codec::Zstd => Box::new(BufReader::new(zstd::Decoder::new(bytes)?)),
      Codec::Bzip2 => Box::new(BufReader::new(bzip2::read::BzDecoder::new(bytes))),
      Codec::None => Box::new(BufReader::new(bytes)),
    })
  }

  /// Decompresses fully into one buffer, for callers that need the whole
  /// payload at once.
  pub fn bytes(self, bytes: &[u8]) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    match self {
      Codec::Gzip => {
        flate2::read::GzDecoder::new(bytes).read_to_end(&mut buf)?;
      }
      Codec::Xz => {
        xz2::read::XzDecoder::new(bytes).read_to_end(&mut buf)?;
      }
      Codec::Zstd => {
        zstd::Decoder::new(bytes)?.read_to_end(&mut buf)?;
      }
      Codec::Bzip2 => {
        bzip2::read::BzDecoder::new(bytes).read_to_end(&mut buf)?;
      }
      Codec::None => {
        buf.extend_from_slice(bytes);
      }
    }
    Ok(buf)
  }
}
