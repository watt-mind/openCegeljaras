//! The pinned schema manifest, its strict types, and its validation rules.
//!
//! The manifest is research metadata: pinned URLs, byte counts, digests, and
//! the import graph of the official 2026 schemas. No schema text is vendored
//! here, and parsing this file establishes nothing about any local mirror.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// The manifest compiled into this build.
const SCHEMA_MANIFEST_JSON: &str = include_str!("../../../../data/schema-manifest.json");

/// The only hosts a pinned resource URL may name.
pub const ALLOWED_HOSTS: [&str; 2] = ["schema.e-cegjegyzek.hu", "www.e-cegjegyzek.hu"];
/// The largest accepted length of one resource URL, in bytes.
const MAX_URL_BYTES: usize = 512;
/// The length of a hexadecimal SHA-256 digest.
const DIGEST_CHARACTERS: usize = 64;

/// The pinned import graph of the official schemas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaManifest {
    /// Version of this manifest's own layout.
    pub snapshot_version: u32,
    /// Date on which the pinned metadata was gathered.
    pub retrieved_at: String,
    /// What the manifest is and is not, in the maintainer's words.
    pub purpose: String,
    /// The graph roots: the form root first, the dossier root second.
    pub roots: Vec<String>,
    /// Every pinned resource, in manifest order.
    pub resources: Vec<SchemaResource>,
}

/// One pinned schema resource. The bytes themselves are not part of this.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaResource {
    /// The pinned absolute URL.
    pub url: String,
    /// The pinned size in bytes.
    pub bytes: u64,
    /// The pinned SHA-256 digest, as 64 lowercase hexadecimal characters.
    pub sha256: String,
    /// The declared target namespace, or `None` when there is none.
    pub target_namespace: Option<String>,
    /// The resource URLs this resource imports or includes.
    pub dependencies: Vec<String>,
    /// The global element names the resource declares.
    pub roots: Vec<String>,
}

/// Why a manifest is not usable. Every variant reports as `manifest_invalid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ManifestProblem {
    /// The text is not the strict manifest JSON shape.
    Syntax,
    /// A resource URL violates the pinned path rules.
    ResourceUrl,
    /// Two resources share one URL.
    DuplicateUrl,
    /// A dependency names a URL that is not a resource.
    UnknownDependency,
    /// A digest is not 64 lowercase hexadecimal characters.
    Digest,
    /// The roots are empty or name something that is not a resource.
    Roots,
}

/// Where a resource URL maps to inside a bundle directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundlePath<'url> {
    /// The URL host, which is the bundle's first path component.
    pub host: &'url str,
    /// The URL path segments, each already checked against the path rules.
    pub segments: Vec<&'url str>,
}

/// Check one URL against the pinned path rules and split it for a bundle.
///
/// The rules exist so that a manifest, compiled or supplied, can never send a
/// read outside the bundle directory: the scheme, both hosts, and every path
/// character are fixed, and `.` and `..` segments are refused outright.
#[must_use]
pub fn bundle_path(url: &str) -> Option<BundlePath<'_>> {
    if url.len() > MAX_URL_BYTES || !url.is_ascii() {
        return None;
    }
    let rest = url.strip_prefix("https://")?;
    let (host, path) = rest.split_once('/')?;
    if !ALLOWED_HOSTS.contains(&host) {
        return None;
    }
    let mut segments = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return None;
        }
        if !segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        {
            return None;
        }
        segments.push(segment);
    }
    Some(BundlePath { host, segments })
}

/// Normalise an `http` URL on the two allowed hosts to `https`.
///
/// The manifest records both hosts as `https`; the published schemas reach
/// some of them through `http`. No other host is ever rewritten.
#[must_use]
pub fn normalise_scheme(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") {
        let host = rest.split('/').next().unwrap_or(rest);
        if ALLOWED_HOSTS.contains(&host) {
            return format!("https://{rest}");
        }
    }
    url.to_owned()
}

impl SchemaManifest {
    /// Parse and validate one manifest text.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestProblem::Syntax`] when the text is not the strict
    /// manifest shape, and the matching rule variant when it is but the
    /// pinned URL, digest, dependency, or root rules do not hold.
    pub fn parse(text: &str) -> Result<Self, ManifestProblem> {
        let manifest: Self = serde_json::from_str(text).map_err(|_| ManifestProblem::Syntax)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Check the pinned URL, digest, dependency, and root rules.
    fn validate(&self) -> Result<(), ManifestProblem> {
        let mut urls = BTreeSet::new();
        for resource in &self.resources {
            if bundle_path(&resource.url).is_none() {
                return Err(ManifestProblem::ResourceUrl);
            }
            if !urls.insert(resource.url.as_str()) {
                return Err(ManifestProblem::DuplicateUrl);
            }
            if resource.sha256.len() != DIGEST_CHARACTERS
                || !resource
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(ManifestProblem::Digest);
            }
        }
        for resource in &self.resources {
            if resource
                .dependencies
                .iter()
                .any(|dependency| !urls.contains(dependency.as_str()))
            {
                return Err(ManifestProblem::UnknownDependency);
            }
        }
        if self.roots.is_empty() || self.roots.iter().any(|root| !urls.contains(root.as_str())) {
            return Err(ManifestProblem::Roots);
        }
        Ok(())
    }

    /// Look one resource up by its pinned URL.
    #[must_use]
    pub fn resource(&self, url: &str) -> Option<&SchemaResource> {
        self.resources.iter().find(|resource| resource.url == url)
    }

    /// The resources reachable from the form root through `dependencies`.
    ///
    /// This is computed from the manifest alone; nothing is read or fetched.
    #[must_use]
    pub fn form_closure(&self) -> BTreeSet<&str> {
        let mut reached = BTreeSet::new();
        let Some(root) = self.roots.first() else {
            return reached;
        };
        let mut pending = vec![root.as_str()];
        while let Some(url) = pending.pop() {
            let Some(resource) = self.resource(url) else {
                continue;
            };
            if !reached.insert(resource.url.as_str()) {
                continue;
            }
            pending.extend(
                resource
                    .dependencies
                    .iter()
                    .map(std::string::String::as_str),
            );
        }
        reached
    }
}

/// Parse the schema manifest compiled into this build.
///
/// # Errors
///
/// Returns the rule violation when the compiled manifest is not valid, which
/// a unit test in this crate rules out for every published build.
pub fn compiled_manifest() -> Result<SchemaManifest, ManifestProblem> {
    SchemaManifest::parse(SCHEMA_MANIFEST_JSON)
}
