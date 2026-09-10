//! Plain RFC 3986 reference resolution for schema locations.
//!
//! Only the strict algorithm of RFC 3986 section 5.2 is implemented, over
//! already-decoded text. Nothing here percent-decodes, normalises a host,
//! consults a catalogue, or touches the network: a resolved reference is a
//! string that is afterwards compared against the pinned manifest, and a
//! reference that resolves to anything unpinned simply fails that comparison.

/// The parts of a URI reference, as split by RFC 3986 appendix B.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Parts<'reference> {
    scheme: Option<&'reference str>,
    authority: Option<&'reference str>,
    path: &'reference str,
    query: Option<&'reference str>,
    fragment: Option<&'reference str>,
}

/// Split a URI reference into its five syntactic parts.
fn split(reference: &str) -> Parts<'_> {
    let (rest, fragment) = match reference.split_once('#') {
        Some((rest, fragment)) => (rest, Some(fragment)),
        None => (reference, None),
    };
    let (rest, query) = match rest.split_once('?') {
        Some((rest, query)) => (rest, Some(query)),
        None => (rest, None),
    };
    let (scheme, rest) = match scheme_end(rest) {
        Some(end) => (Some(&rest[..end]), &rest[end + 1..]),
        None => (None, rest),
    };
    let (authority, path) = match rest.strip_prefix("//") {
        Some(rest) => {
            let end = rest.find('/').unwrap_or(rest.len());
            (Some(&rest[..end]), &rest[end..])
        }
        None => (None, rest),
    };
    Parts {
        scheme,
        authority,
        path,
        query,
        fragment,
    }
}

/// The index of the colon ending a syntactically valid scheme, if any.
fn scheme_end(rest: &str) -> Option<usize> {
    let end = rest.find(':')?;
    let scheme = &rest[..end];
    let mut characters = scheme.chars();
    if !characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
    {
        return None;
    }
    characters
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.'))
        .then_some(end)
}

/// Apply RFC 3986 section 5.2.4, removing `.` and `..` segments.
fn remove_dot_segments(path: &str) -> String {
    let mut output: Vec<&str> = Vec::new();
    let absolute = path.starts_with('/');
    let mut trailing = false;
    for segment in path.split('/') {
        match segment {
            "." => trailing = true,
            ".." => {
                output.pop();
                trailing = true;
            }
            "" => trailing = true,
            other => {
                output.push(other);
                trailing = false;
            }
        }
    }
    let mut result = String::new();
    for segment in &output {
        result.push('/');
        result.push_str(segment);
    }
    if trailing && !output.is_empty() {
        result.push('/');
    }
    if !absolute {
        result = result.strip_prefix('/').unwrap_or(&result).to_owned();
    }
    if absolute && result.is_empty() {
        result.push('/');
    }
    result
}

/// Merge a relative path with a base, as RFC 3986 section 5.2.3 defines.
fn merge(base: Parts<'_>, path: &str) -> String {
    if base.authority.is_some() && base.path.is_empty() {
        return format!("/{path}");
    }
    let keep = base.path.rfind('/').map_or(0, |index| index + 1);
    format!("{}{path}", &base.path[..keep])
}

/// Resolve `reference` against an absolute `base`, strictly.
///
/// Returns `None` when `base` is not an absolute URI, which the manifest path
/// rules already exclude for every pinned resource URL.
pub(crate) fn resolve(base: &str, reference: &str) -> Option<String> {
    let base = split(base);
    let scheme = base.scheme?;
    let reference = split(reference);

    if reference.scheme.is_some() || reference.authority.is_some() {
        let path = remove_dot_segments(reference.path);
        return Some(
            Parts {
                scheme: reference.scheme.or(Some(scheme)),
                path: &path,
                ..reference
            }
            .recompose(),
        );
    }

    let target = if reference.path.is_empty() {
        Parts {
            scheme: Some(scheme),
            authority: base.authority,
            path: base.path,
            query: reference.query.or(base.query),
            fragment: reference.fragment,
        }
        .recompose()
    } else {
        let path = if reference.path.starts_with('/') {
            remove_dot_segments(reference.path)
        } else {
            remove_dot_segments(&merge(base, reference.path))
        };
        Parts {
            scheme: Some(scheme),
            authority: base.authority,
            path: &path,
            query: reference.query,
            fragment: reference.fragment,
        }
        .recompose()
    };
    Some(target)
}

impl Parts<'_> {
    /// Recompose the parts into one string, as RFC 3986 section 5.3 defines.
    fn recompose(self) -> String {
        let mut result = String::new();
        if let Some(scheme) = self.scheme {
            result.push_str(scheme);
            result.push(':');
        }
        if let Some(authority) = self.authority {
            result.push_str("//");
            result.push_str(authority);
        }
        result.push_str(self.path);
        if let Some(query) = self.query {
            result.push('?');
            result.push_str(query);
        }
        if let Some(fragment) = self.fragment {
            result.push('#');
            result.push_str(fragment);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/root.xsd";

    #[test]
    fn relative_references_resolve_against_the_base_directory() {
        for (reference, expected) in [
            (
                "child.xsd",
                "https://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/child.xsd",
            ),
            (
                "./child.xsd",
                "https://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/child.xsd",
            ),
            (
                "../other/child.xsd",
                "https://schema.e-cegjegyzek.hu/schema/other/child.xsd",
            ),
            (
                "../../../child.xsd",
                "https://schema.e-cegjegyzek.hu/child.xsd",
            ),
            (
                "/absolute/child.xsd",
                "https://schema.e-cegjegyzek.hu/absolute/child.xsd",
            ),
            (
                "//www.e-cegjegyzek.hu/a/b.xsd",
                "https://www.e-cegjegyzek.hu/a/b.xsd",
            ),
            (
                "http://www.w3.org/2001/03/xml.xsd",
                "http://www.w3.org/2001/03/xml.xsd",
            ),
            (BASE, BASE),
        ] {
            assert_eq!(resolve(BASE, reference).as_deref(), Some(expected));
        }
    }

    #[test]
    fn queries_and_fragments_survive_resolution() {
        assert_eq!(
            resolve(BASE, "child.xsd?v=1#top").as_deref(),
            Some("https://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/child.xsd?v=1#top")
        );
        assert_eq!(resolve(BASE, "").as_deref(), Some(BASE));
        assert_eq!(
            resolve(BASE, "#top").as_deref(),
            Some("https://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/root.xsd#top")
        );
    }

    #[test]
    fn a_directory_reference_keeps_its_trailing_separator() {
        assert_eq!(
            resolve(BASE, "sub/").as_deref(),
            Some("https://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/sub/")
        );
        assert_eq!(
            resolve("https://schema.e-cegjegyzek.hu", "a.xsd").as_deref(),
            Some("https://schema.e-cegjegyzek.hu/a.xsd")
        );
    }

    #[test]
    fn a_base_without_a_scheme_resolves_nothing() {
        assert_eq!(resolve("/schema/root.xsd", "child.xsd"), None);
        assert_eq!(scheme_end("1http://a"), None);
        assert_eq!(scheme_end("ht tp://a"), None);
    }
}
