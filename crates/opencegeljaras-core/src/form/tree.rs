//! Bounded tree parsing, structural counting, and 2026 form classification.

use std::collections::BTreeSet;

use roxmltree::{Document, Node, ParsingOptions};

use crate::form::{
    FormFacts, FormStatus, Limits, Rejection, RootFacts, Section, StructureFacts, Variant,
    truncate_name,
};

/// The XML Schema instance namespace whose location hints are never resolved.
pub(crate) const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";
/// Common prefix of every company-form namespace.
const NAMESPACE_ROOT: &str = "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/";
/// Local name of the wrapper element.
const WRAPPER_LOCAL_NAME: &str = "Nyomtatvány";
/// Namespace of the supported 2026 wrapper.
pub const WRAPPER_NAMESPACE: &str =
    "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20260101#";
/// Profile identifier reported for the supported wrapper.
pub const PROFILE: &str = "form-2026";
/// The largest number of distinct section names or namespaces reported.
const REPORT_CAP: usize = 64;

/// The three known variant roots: identifier, local name, and namespace.
const VARIANTS: [(&str, &str, &str); 3] = [
    (
        "cegbejegyzesi-kerelem-20260101",
        "CégbejegyzésiKérelem",
        "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegbejegyzesiKerelem-20260101#",
    ),
    (
        "valtozasbejegyzesi-kerelem-20240101",
        "VáltozásbejegyzésiKérelem",
        "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/valtozasbejegyzesiKerelem-20240101#",
    ),
    (
        "nevfoglalasi-kerelem-20210401",
        "NévfoglalásiKérelem",
        "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/nevfoglalasiKerelem-20210401#",
    ),
];

/// The parsing options this crate uses, with entity resolution disabled.
pub(crate) fn parsing_options<'input>(limits: &Limits) -> ParsingOptions<'input> {
    ParsingOptions {
        allow_dtd: false,
        nodes_limit: limits.parser_nodes(),
        entity_resolver: None,
    }
}

/// Parse pre-scanned text, mapping parser errors to fixed rejection codes.
pub(crate) fn parse<'text>(
    text: &'text str,
    limits: &Limits,
) -> Result<Document<'text>, Rejection> {
    Document::parse_with_options(text, parsing_options(limits)).map_err(rejection_for)
}

/// Map a parser error to its rejection code, dropping every position.
fn rejection_for(error: roxmltree::Error) -> Rejection {
    match error {
        roxmltree::Error::DtdDetected => Rejection::ProhibitedConstruct,
        roxmltree::Error::NodesLimitReached
        | roxmltree::Error::AttributesLimitReached
        | roxmltree::Error::NamespacesLimitReached => Rejection::LimitExceeded,
        _ => Rejection::MalformedXml,
    }
}

/// Whether a stylesheet instruction appears before the root element.
pub(crate) fn stylesheet_pi(document: &Document<'_>) -> bool {
    document
        .root()
        .children()
        .take_while(|node| !node.is_element())
        .filter_map(|node| node.pi())
        .any(|pi| pi.target == "xml-stylesheet")
}

/// Walk the parsed tree, enforcing the text and attribute limits.
pub(crate) fn walk(
    document: &Document<'_>,
    limits: &Limits,
) -> Result<(StructureFacts, bool), Rejection> {
    let mut elements = 0usize;
    let mut attributes = 0usize;
    let mut text_bytes = 0usize;
    let mut max_depth = 0usize;
    let mut schema_location = false;
    let mut namespaces = BTreeSet::new();

    let mut stack = vec![(document.root_element(), 1usize)];
    while let Some((node, depth)) = stack.pop() {
        if node.is_element() {
            elements += 1;
            max_depth = max_depth.max(depth);
            if elements > limits.elements || depth > limits.depth {
                return Err(Rejection::LimitExceeded);
            }
            let count = node.attributes().count();
            if count > limits.attributes {
                return Err(Rejection::LimitExceeded);
            }
            attributes += count;
            schema_location |= declares_schema_location(node);
            if let Some(namespace) = node.tag_name().namespace() {
                namespaces.insert(namespace);
            }
            stack.extend(node.children().map(|child| (child, depth + 1)));
        } else if let Some(text) = node.text().filter(|_| node.is_text()) {
            if text.len() > limits.text_bytes {
                return Err(Rejection::LimitExceeded);
            }
            text_bytes += text.len();
        }
    }

    let namespaces_truncated = namespaces.len() > REPORT_CAP;
    Ok((
        StructureFacts {
            elements,
            max_depth,
            attributes,
            text_bytes,
            namespaces: namespaces
                .into_iter()
                .take(REPORT_CAP)
                .map(truncate_name)
                .collect(),
            namespaces_truncated,
        },
        schema_location,
    ))
}

/// Whether an element carries an XSI schema-location hint. Values are unread.
fn declares_schema_location(node: Node<'_, '_>) -> bool {
    node.attributes().any(|attribute| {
        attribute.namespace() == Some(XSI_NAMESPACE)
            && matches!(
                attribute.name(),
                "schemaLocation" | "noNamespaceSchemaLocation"
            )
    })
}

/// Facts about the root element, with document strings cut to a fixed length.
pub(crate) fn root_facts(document: &Document<'_>) -> RootFacts {
    let root = document.root_element();
    RootFacts {
        local_name: truncate_name(root.tag_name().name()),
        namespace: root.tag_name().namespace().map(truncate_name),
    }
}

/// Classify the document as a supported form, or say why it is not one.
pub(crate) fn classify(document: &Document<'_>) -> FormFacts {
    let root = document.root_element();
    let name = root.tag_name();
    if name.name() != WRAPPER_LOCAL_NAME {
        return FormFacts::unrecognized(FormStatus::UnrecognizedRoot);
    }
    match name.namespace() {
        Some(WRAPPER_NAMESPACE) => classify_variant(root),
        Some(namespace) if is_wrapper_version(namespace) => {
            FormFacts::unrecognized(FormStatus::UnsupportedVersion)
        }
        _ => FormFacts::unrecognized(FormStatus::UnrecognizedRoot),
    }
}

/// Whether a namespace is a differently dated wrapper namespace.
fn is_wrapper_version(namespace: &str) -> bool {
    let Some(rest) = namespace.strip_prefix(NAMESPACE_ROOT) else {
        return false;
    };
    let Some(date) = rest
        .strip_prefix("cegnyomtatvany-")
        .and_then(|rest| rest.strip_suffix('#'))
    else {
        return false;
    };
    date.len() == 8 && date.bytes().all(|byte| byte.is_ascii_digit())
}

/// Identify the single variant child of a 2026 wrapper.
fn classify_variant(root: Node<'_, '_>) -> FormFacts {
    let mut children = root.children().filter(Node::is_element);
    let (Some(child), None) = (children.next(), children.next()) else {
        return FormFacts::wrapper_only(FormStatus::UnrecognizedVariant, PROFILE);
    };
    let name = child.tag_name();
    let Some(&(id, _, namespace)) = VARIANTS.iter().find(|(_, local, namespace)| {
        name.name() == *local && name.namespace() == Some(*namespace)
    }) else {
        return FormFacts::wrapper_only(FormStatus::UnrecognizedVariant, PROFILE);
    };

    let (sections, sections_truncated) = sections(child);
    FormFacts {
        status: FormStatus::Recognized,
        profile: Some(PROFILE),
        variant: Some(Variant {
            id,
            local_name: truncate_name(name.name()),
            namespace: Some(truncate_name(namespace)),
        }),
        sections: Some(sections),
        sections_truncated,
    }
}

/// Direct element children of the variant root, in first-appearance order.
///
/// Names are counted whole and cut only when they are reported, so two
/// distinct long names never merge into one section.
fn sections(variant: Node<'_, '_>) -> (Vec<Section>, bool) {
    let mut counted: Vec<(&str, usize)> = Vec::new();
    let mut truncated = false;
    for child in variant.children().filter(Node::is_element) {
        let local_name = child.tag_name().name();
        if let Some((_, count)) = counted.iter_mut().find(|(name, _)| *name == local_name) {
            *count += 1;
        } else if counted.len() == REPORT_CAP {
            truncated = true;
        } else {
            counted.push((local_name, 1));
        }
    }
    let sections = counted
        .into_iter()
        .map(|(local_name, count)| Section {
            local_name: truncate_name(local_name),
            count,
        })
        .collect();
    (sections, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_options_disable_dtds_and_entity_resolution() {
        let limits = Limits::default();
        let options = parsing_options(&limits);
        assert!(!options.allow_dtd);
        assert_eq!(options.nodes_limit, 800_000);
        assert_eq!(options.nodes_limit, limits.parser_nodes());
        assert!(options.entity_resolver.is_none());
    }

    #[test]
    fn parser_errors_map_to_fixed_codes() {
        assert_eq!(
            rejection_for(roxmltree::Error::DtdDetected),
            Rejection::ProhibitedConstruct
        );
        for error in [
            roxmltree::Error::NodesLimitReached,
            roxmltree::Error::AttributesLimitReached,
            roxmltree::Error::NamespacesLimitReached,
        ] {
            assert_eq!(rejection_for(error), Rejection::LimitExceeded);
        }
        for error in [
            roxmltree::Error::EntityReferenceLoop(roxmltree::TextPos::new(1, 1)),
            roxmltree::Error::UnknownEntityReference(
                "foo".to_owned(),
                roxmltree::TextPos::new(1, 1),
            ),
            roxmltree::Error::NoRootNode,
            roxmltree::Error::UnclosedRootNode,
        ] {
            assert_eq!(rejection_for(error), Rejection::MalformedXml);
        }
    }

    #[test]
    fn a_differently_dated_wrapper_namespace_is_recognised_as_a_version() {
        assert!(is_wrapper_version(
            "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20240101#"
        ));
        assert!(is_wrapper_version(WRAPPER_NAMESPACE));
        for namespace in [
            "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-2024#",
            "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-2024010a#",
            "http://schema.e-cegjegyzek.hu/schema/cegnyomtatvany/cegnyomtatvany-20240101",
            "http://example.invalid/cegnyomtatvany-20240101#",
        ] {
            assert!(!is_wrapper_version(namespace), "{namespace}");
        }
    }
}
