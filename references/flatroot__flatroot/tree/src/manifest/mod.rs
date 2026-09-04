//! The `.flatroot/` metadata a rootfs carries: what was installed, from
//! which distribution and release, and the checksums that verify each
//! piece is the byte-for-byte original.

mod checksum;
mod codec;
mod layout;
mod record;
mod state;

pub use checksum::{Checksum, ChecksumAlgorithm};
pub use codec::ManifestCodec;
pub use layout::ManifestLayout;
pub use record::PackageRecord;
pub use state::ManifestState;
