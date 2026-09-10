//! The import audit: which schemas a pinned resource actually reaches.
//!
//! The audit answers one question about one already-read resource: does every
//! `xs:import` and `xs:include` it declares resolve to a pinned manifest URL,
//! with the namespace the manifest records? Nothing is fetched, and no
//! location the resource names is ever opened.

use std::collections::BTreeSet;

use roxmltree::{Document, Node};

use crate::form::{self, Failure, Limits};
use crate::schema::manifest::{SchemaManifest, normalise_scheme};
use crate::schema::{LOCATION_CAP, LOCATION_LIMIT, uri};

/// The XML Schema namespace whose import and include elements are audited.
const XSD_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";

/// Whether an edge carries a namespace the manifest must agree with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// An `xs:import`, which names the namespace it expects.
    Import,
    /// An `xs:include`, which has no namespace of its own.
    Include,
}

/// The offending locations of one resource, cut to the reported maximum.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnpinnedLocations {
    /// The distinct offending locations, in first-appearance order.
    pub locations: Vec<String>,
    /// Whether more distinct offending locations were found than are listed.
    pub truncated: bool,
}

/// One `xs:import` or `xs:include` edge of a schema resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportEdge {
    /// Whether the namespace attribute has to agree with the manifest.
    pub kind: EdgeKind,
    /// The declared `namespace` attribute, if any.
    pub namespace: Option<String>,
    /// The `schemaLocation` resolved against the resource URL, if it was
    /// declared at all. An edge without a location pins nothing.
    pub resolved: Option<String>,
}

/// Collect every import and include edge of one already-read resource.
///
/// The bytes go through the same bounded parsing entry point as a form
/// document, so an XSD with a DTD, an unsupported encoding, or a construct
/// past a fixed limit is refused instead of audited.
///
/// # Errors
///
/// Returns the parse [`Failure`] when the resource does not pass that entry
/// point; the caller reports such a resource as `unparseable`.
pub fn edges(bytes: &[u8], base_url: &str, limits: &Limits) -> Result<Vec<ImportEdge>, Failure> {
    form::parse_bounded(bytes, limits, |document, _| Ok(collect(document, base_url)))
}

/// Read the edges out of a parsed schema document.
fn collect(document: &Document<'_>, base_url: &str) -> Vec<ImportEdge> {
    document
        .descendants()
        .filter(Node::is_element)
        .filter_map(|node| {
            let name = node.tag_name();
            if name.namespace() != Some(XSD_NAMESPACE) {
                return None;
            }
            let kind = match name.name() {
                "import" => EdgeKind::Import,
                "include" => EdgeKind::Include,
                _ => return None,
            };
            Some(ImportEdge {
                kind,
                namespace: node.attribute("namespace").map(str::to_owned),
                resolved: node
                    .attribute("schemaLocation")
                    .and_then(|location| uri::resolve(base_url, location))
                    .as_deref()
                    .map(normalise_scheme),
            })
        })
        .collect()
}

/// The offending locations of one resource's edges, in first-appearance order.
///
/// An edge offends when it declares no location, when its resolved location is
/// not a pinned resource, or when an import's namespace is not the namespace
/// the manifest records for the resource it resolves to. Locations are
/// bundle-derived strings, so each is cut to a fixed number of Unicode scalar
/// values, de-duplicated, and reported at most 64 to one entry.
#[must_use]
pub fn unpinned_locations(manifest: &SchemaManifest, edges: &[ImportEdge]) -> UnpinnedLocations {
    let mut found = UnpinnedLocations::default();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for edge in edges {
        let location = match edge.resolved.as_deref() {
            // An edge without a `schemaLocation` pins nothing at all; it is
            // reported as the empty location, which no resolution produces.
            None => String::new(),
            Some(location) => {
                let target = manifest.resource(location);
                if target.is_some_and(|resource| {
                    edge.kind == EdgeKind::Include
                        || resource.target_namespace.as_deref() == edge.namespace.as_deref()
                }) {
                    continue;
                }
                crate::truncate_scalars(location, LOCATION_LIMIT)
            }
        };
        if !seen.insert(location.clone()) {
            continue;
        }
        if found.locations.len() < LOCATION_CAP {
            found.locations.push(location);
        } else {
            found.truncated = true;
        }
    }
    found
}
