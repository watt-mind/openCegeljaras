//! The pinned schema manifest, bundle digesting, and the import audit.
//!
//! This module reads no file, opens no socket, spawns nothing, and asks for no
//! clock. It parses the compiled manifest or a manifest text a caller supplies,
//! digests bytes a caller has already read, and audits the imports of one
//! already-read schema resource. Everything it establishes is a statement about
//! bytes and pinned metadata: never a redistribution licence, never XSD
//! validation, and never acceptance by any authority.

mod imports;
mod manifest;
mod uri;

use sha2::{Digest, Sha256};

pub use imports::{EdgeKind, ImportEdge, edges, unpinned_locations};
pub use manifest::{
    ALLOWED_HOSTS, BundlePath, ManifestProblem, SchemaManifest, SchemaResource, bundle_path,
    compiled_manifest, normalise_scheme,
};

/// The fixed notice attached to every `schemas list` result.
pub const LIST_NOTICE: &str = "pinned research metadata; nothing was fetched or verified";
/// The fixed notice attached to every `schemas verify` result.
pub const VERIFY_NOTICE: &str =
    "digest and import audit of a local mirror; not a licence, not validation";
/// Appended to the verify notice when the manifest itself came from a file.
pub const UNPINNED_MANIFEST_NOTICE: &str = "; unpinned manifest";
/// The largest bundle file or manifest file that is read, in bytes.
///
/// The largest pinned resource is under 100 KiB; this cap bounds the work, not
/// expectations about the schemas.
pub const MAX_RESOURCE_BYTES: u64 = 4 * 1024 * 1024;
/// The largest number of Unicode scalar values kept from a bundle string.
const LOCATION_LIMIT: usize = 128;

/// What one bundle resource turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceStatus {
    /// The bytes match the pinned size and digest, and the imports audited.
    Verified,
    /// No readable regular file is mirrored at the resource's bundle path.
    Missing,
    /// The mirrored file is larger than the read cap.
    Oversize,
    /// The mirrored bytes do not match the pinned size or digest.
    Mismatch,
    /// A form-closure resource did not pass the bounded parsing entry point.
    Unparseable,
    /// A form-closure resource imports something the manifest does not pin.
    UnpinnedImport,
}

impl ResourceStatus {
    /// The stable machine-readable status.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Missing => "missing",
            Self::Oversize => "oversize",
            Self::Mismatch => "mismatch",
            Self::Unparseable => "unparseable",
            Self::UnpinnedImport => "unpinned_import",
        }
    }
}

/// The lowercase hexadecimal SHA-256 digest of already-read bytes.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let mut digest = String::with_capacity(64);
    for byte in hasher.finalize() {
        digest.push_str(&format!("{byte:02x}"));
    }
    digest
}

#[cfg(test)]
mod tests;
