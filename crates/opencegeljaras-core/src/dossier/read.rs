//! Reading one parsed dossier tree into the reported facts.
//!
//! Every lookup is by local name in the dossier's own namespace and takes the
//! first matching child; extras are left to `xsd_conformance` rather than
//! guessed at. A `ds:Object` belongs to the `Document` that contains it and to
//! nothing else: `OBJREF`, `Id`, `SIGREF`, and `SIGREFLIST` are never
//! resolved. Only one document's payload is held at a time.

use roxmltree::{Document, Node};

use super::FormOutcome;
use super::base64;
use super::patterns::{
    DOCUMENT_CATEGORIES, DOSSIER_CATEGORIES, PROCEDURE_TITLES, RECEIPT_CATEGORIES,
};
use super::roles;
use super::{
    DOCUMENT_LIMIT, DocumentFacts, DossierFacts, DossierStatus, Identity, InputFacts, Inspection,
    Mime, NAMESPACE, PROFILE, PayloadStatus, TOKEN_LIMIT, VALUE_LIMIT,
};
use crate::form::{self, BoundedInput, Failure, Limits};
use crate::truncate_scalars;

/// The XMLDSig namespace of `ds:Object` and `ds:Signature`.
const DSIG_NAMESPACE: &str = "http://www.w3.org/2000/09/xmldsig#";
/// The fixed identifier the `Documents` container must carry.
const DOCUMENTS_ID: &str = "Object0";
/// The supported transform algorithms, in the schema's own order.
const ALGORITHMS: [&str; 3] = ["zip", "encrypt", "base64"];
/// Reported for a transform whose algorithm is absent or unrecognised.
const UNKNOWN_ALGORITHM: &str = "unknown";
/// Media subtype that declares a nested dossier.
const NESTED_SUBTYPE: &str = "nldossier2";
/// File extension that declares a nested dossier.
const NESTED_EXTENSION: &str = "dosszie";
/// The extension that declares XML content.
const XML_EXTENSION: &str = "xml";

/// Read one parsed dossier under `limits` into the reported facts.
pub(crate) fn read(
    tree: &Document<'_>,
    input: BoundedInput,
    limits: &Limits,
) -> Result<Inspection, Failure> {
    let root = tree.root_element();
    let facts = InputFacts {
        bytes: input.bytes,
        bom: input.bom,
        xml_declaration: input.xml_declaration,
        encoding: input.encoding,
    };
    form::walk(tree, limits).map_err(|code| input.failure(code))?;
    let max_text_bytes = widest_text(tree);
    let status = classify(root);
    let root_facts = form::root_facts(tree);
    if status != DossierStatus::Recognized {
        return Ok(Inspection {
            input: facts,
            root: root_facts,
            dossier: unrecognized(status),
            documents: None,
            documents_truncated: false,
            signatures_present: 0,
            timestamps_present: 0,
            max_text_bytes,
        });
    }
    let (documents, truncated) = documents(root);
    let signatures = counted(root, DSIG_NAMESPACE, "Signature")
        + documents.iter().map(|item| item.signatures).sum::<usize>();
    let timestamps = counted(root, NAMESPACE, "TimeStamp")
        + documents.iter().map(|item| item.timestamps).sum::<usize>();
    Ok(Inspection {
        input: facts,
        root: root_facts,
        dossier: profile_facts(root),
        documents: Some(documents),
        documents_truncated: truncated,
        signatures_present: signatures,
        timestamps_present: timestamps,
        max_text_bytes,
    })
}

/// Classify the root element against the supported dossier namespace.
fn classify(root: Node<'_, '_>) -> DossierStatus {
    let name = root.tag_name();
    if name.name() != "Dossier" {
        return DossierStatus::UnrecognizedRoot;
    }
    match name.namespace() {
        Some(NAMESPACE) => DossierStatus::Recognized,
        Some(namespace) if is_dossier_version(namespace) => DossierStatus::UnsupportedVersion,
        _ => DossierStatus::UnrecognizedRoot,
    }
}

/// Whether a namespace is a differently dated dossier namespace.
fn is_dossier_version(namespace: &str) -> bool {
    let Some(rest) = namespace.strip_prefix("http://www.e-cegjegyzek.hu/") else {
        return false;
    };
    let Some(year) = rest.strip_suffix("/e-cegeljaras#") else {
        return false;
    };
    year.len() == 4 && year.bytes().all(|byte| byte.is_ascii_digit())
}

/// The facts of a document that is not a recognised dossier.
fn unrecognized(status: DossierStatus) -> DossierFacts {
    DossierFacts {
        status,
        profile: None,
        title: None,
        category: None,
        receipt: false,
        creation_date: None,
        ack_email_present: false,
        location_present: false,
        identity: Identity::default(),
        documents_id_fixed: false,
    }
}

/// The largest single element's text content, in bytes.
///
/// libxml2 accumulates one element's character data into one node, so this is
/// what the package 4 validator's own text-node ceiling applies to.
fn widest_text(tree: &Document<'_>) -> usize {
    tree.descendants()
        .filter(Node::is_element)
        .map(|node| {
            node.children()
                .filter(Node::is_text)
                .filter_map(|child| child.text())
                .map(str::len)
                .sum::<usize>()
        })
        .max()
        .unwrap_or_default()
}

/// The first child element with one local name in one namespace.
fn child<'a>(node: Node<'a, 'a>, namespace: &str, local: &str) -> Option<Node<'a, 'a>> {
    node.children().find(|child| {
        child.is_element()
            && child.tag_name().name() == local
            && child.tag_name().namespace() == Some(namespace)
    })
}

/// How many child elements carry one local name in one namespace.
fn counted(node: Node<'_, '_>, namespace: &str, local: &str) -> usize {
    node.children()
        .filter(|child| {
            child.is_element()
                && child.tag_name().name() == local
                && child.tag_name().namespace() == Some(namespace)
        })
        .count()
}

/// One element's base64 text with XML whitespace removed, in one pass.
///
/// The text children are never concatenated into a string of their own, so a
/// 64 MiB payload costs one buffer rather than two.
fn packed_base64(node: Node<'_, '_>) -> Vec<u8> {
    let mut packed = Vec::new();
    for text in node
        .children()
        .filter(Node::is_text)
        .filter_map(|child| child.text())
    {
        base64::absorb(text, &mut packed);
    }
    packed
}

/// The concatenated text of one element, ignoring comments and instructions.
fn text_of(node: Node<'_, '_>) -> String {
    node.children()
        .filter(Node::is_text)
        .filter_map(|child| child.text())
        .collect()
}

/// The text of one dossier-namespace child element, when it has one.
fn child_text(node: Node<'_, '_>, local: &str) -> Option<String> {
    child(node, NAMESPACE, local).map(text_of)
}

/// The value of one unnamespaced attribute.
fn attribute<'a>(node: Node<'a, 'a>, name: &str) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| attribute.namespace().is_none() && attribute.name() == name)
        .map(|attribute| attribute.value())
}

/// The one enumerated value a declared string equals, if any.
fn enumerated(declared: Option<&str>, values: &[&'static str]) -> Option<&'static str> {
    let declared = declared?;
    values.iter().copied().find(|value| *value == declared)
}

/// The dossier profile's account of the procedure.
fn profile_facts(root: Node<'_, '_>) -> DossierFacts {
    let profile = child(root, NAMESPACE, "DossierProfile");
    let title = profile
        .and_then(|node| child_text(node, "Title"))
        .and_then(|declared| enumerated(Some(&declared), &PROCEDURE_TITLES));
    let category = profile
        .and_then(|node| child_text(node, "E-category"))
        .and_then(|declared| enumerated(Some(&declared), &DOSSIER_CATEGORIES));
    let creation_date = profile
        .and_then(|node| child_text(node, "CreationDate"))
        .filter(|declared| is_date_time(declared));
    let identity = Identity {
        cegforma: profile.and_then(|node| identity_value(node, "Cégforma")),
        cegjegyzekszam: profile.and_then(|node| identity_value(node, "Cégjegyzékszám")),
        cegnev: profile.and_then(|node| identity_value(node, "Cégnév")),
        cegszekhely: profile.and_then(|node| identity_value(node, "Cégszékhely")),
    };
    DossierFacts {
        status: DossierStatus::Recognized,
        profile: Some(PROFILE),
        title,
        category,
        receipt: category.is_some_and(|value| RECEIPT_CATEGORIES.contains(&value)),
        creation_date,
        ack_email_present: attribute(root, "AckEmail").is_some(),
        location_present: attribute(root, "Location").is_some(),
        identity,
        documents_id_fixed: child(root, NAMESPACE, "Documents")
            .and_then(|node| attribute(node, "Id"))
            .is_some_and(|value| value == DOCUMENTS_ID),
    }
}

/// One company-identity element's value, cut to the value limit.
fn identity_value(profile: Node<'_, '_>, local: &str) -> Option<String> {
    child_text(profile, local).map(|value| truncate_scalars(&value, VALUE_LIMIT))
}

/// Whether a declared string is lexically an `xs:dateTime`.
///
/// This is the lexical shape alone. No calendar arithmetic is performed, and
/// nothing about the value is otherwise assumed.
fn is_date_time(declared: &str) -> bool {
    let rest = declared.strip_prefix('-').unwrap_or(declared);
    let Some((date, time)) = rest.split_once('T') else {
        return false;
    };
    let parts: Vec<&str> = date.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return false;
    };
    if year.len() < 4 || !digits(year) || !fixed_digits(month, 2) || !fixed_digits(day, 2) {
        return false;
    }
    is_time(time)
}

/// Whether a string is a non-empty run of ASCII digits.
fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Whether a string is exactly `count` ASCII digits.
fn fixed_digits(text: &str, count: usize) -> bool {
    text.len() == count && digits(text)
}

/// Whether a string is an `xs:dateTime` time with an optional zone.
fn is_time(time: &str) -> bool {
    let (clock, zone) = split_zone(time);
    let parts: Vec<&str> = clock.split(':').collect();
    let [hour, minute, second] = parts.as_slice() else {
        return false;
    };
    let (whole, fraction) = second.split_once('.').unwrap_or((second, "0"));
    fixed_digits(hour, 2)
        && fixed_digits(minute, 2)
        && fixed_digits(whole, 2)
        && digits(fraction)
        && zone
}

/// Split an optional time zone off a clock, reporting whether it is valid.
fn split_zone(time: &str) -> (&str, bool) {
    if let Some(clock) = time.strip_suffix('Z') {
        return (clock, true);
    }
    let Some(index) = time.rfind(['+', '-']) else {
        return (time, true);
    };
    let offset = &time[index + 1..];
    let valid = offset
        .split_once(':')
        .is_some_and(|(hour, minute)| fixed_digits(hour, 2) && fixed_digits(minute, 2));
    (&time[..index], valid)
}

/// The listed documents, in document order, cut at the document limit.
fn documents(root: Node<'_, '_>) -> (Vec<DocumentFacts>, bool) {
    let Some(container) = child(root, NAMESPACE, "Documents") else {
        return (Vec::new(), false);
    };
    let mut listed = Vec::new();
    let mut truncated = false;
    for (index, node) in container
        .children()
        .filter(|child| {
            child.is_element()
                && child.tag_name().name() == "Document"
                && child.tag_name().namespace() == Some(NAMESPACE)
        })
        .enumerate()
    {
        if listed.len() == DOCUMENT_LIMIT {
            truncated = true;
            break;
        }
        listed.push(document_facts(node, index));
    }
    (listed, truncated)
}

/// One document's declared facts and, when it applies, its payload.
fn document_facts(node: Node<'_, '_>, index: usize) -> DocumentFacts {
    let profile = child(node, NAMESPACE, "DocumentProfile");
    let title = profile.and_then(|profile| child_text(profile, "Title"));
    let mime = profile.map(mime_of).unwrap_or_default();
    let transforms = profile.map(transforms_of).unwrap_or_default();
    let object = child(node, DSIG_NAMESPACE, "Object");
    let mut facts = DocumentFacts {
        index,
        role: title.as_deref().and_then(roles::role_for),
        title: title.map(|value| truncate_scalars(&value, VALUE_LIMIT)),
        category: profile
            .and_then(|profile| child_text(profile, "E-category"))
            .and_then(|declared| enumerated(Some(&declared), &DOCUMENT_CATEGORIES)),
        source_size: profile
            .and_then(|profile| child(profile, NAMESPACE, "SourceSize"))
            .and_then(|node| attribute(node, "sizeValue"))
            .and_then(|value| value.parse::<i64>().ok()),
        object_present: object.is_some(),
        nested_dossier: is_nested(&mime),
        signatures: counted(node, DSIG_NAMESPACE, "Signature"),
        timestamps: counted(node, NAMESPACE, "TimeStamp"),
        payload: PayloadStatus::ObjectMissing,
        form: None,
        mime,
        transforms,
    };
    let (payload, form) = payload_of(object, &facts);
    facts.payload = payload;
    facts.form = form;
    facts
}

/// The declared media type of one document profile.
fn mime_of(profile: Node<'_, '_>) -> Mime {
    let Some(declared) = child(profile, NAMESPACE, "Format")
        .and_then(|format| child(format, NAMESPACE, "MIME-Type"))
    else {
        return Mime::default();
    };
    let token = |name| attribute(declared, name).map(|value| truncate_scalars(value, TOKEN_LIMIT));
    Mime {
        type_name: token("type"),
        subtype: token("subtype"),
        extension: token("extension"),
        charset_present: attribute(declared, "charSet").is_some(),
    }
}

/// The declared transform chain, in order, with unknown algorithms named.
fn transforms_of(profile: Node<'_, '_>) -> Vec<&'static str> {
    let Some(base) = child(profile, NAMESPACE, "BaseTransform") else {
        return Vec::new();
    };
    base.children()
        .filter(|child| {
            child.is_element()
                && child.tag_name().name() == "Transform"
                && child.tag_name().namespace() == Some(NAMESPACE)
        })
        .map(|node| {
            enumerated(attribute(node, "Algorithm"), &ALGORITHMS).unwrap_or(UNKNOWN_ALGORITHM)
        })
        .collect()
}

/// Whether a declared media type names a nested dossier. It is never opened.
fn is_nested(mime: &Mime) -> bool {
    let matches = |declared: &Option<String>, expected: &str| {
        declared
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case(expected))
    };
    matches(&mime.subtype, NESTED_SUBTYPE) || matches(&mime.extension, NESTED_EXTENSION)
}

/// Whether a declared media type names XML content.
///
/// Either `text/xml` or `application/xml` is declared, or the declared file
/// extension is `xml`. Nothing is sniffed and no byte is looked at.
fn is_xml(mime: &Mime) -> bool {
    let equals = |declared: &Option<String>, expected: &str| {
        declared
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case(expected))
    };
    let media = equals(&mime.subtype, XML_EXTENSION)
        && (equals(&mime.type_name, "text") || equals(&mime.type_name, "application"));
    media || equals(&mime.extension, XML_EXTENSION)
}

/// Decide one document's payload, and inspect it when every condition holds.
///
/// The reasons are decided in the contract's order and the first applicable
/// one wins. The decoded bytes live only for this call.
fn payload_of(
    object: Option<Node<'_, '_>>,
    facts: &DocumentFacts,
) -> (PayloadStatus, Option<FormOutcome>) {
    let Some(object) = object else {
        return (PayloadStatus::ObjectMissing, None);
    };
    if object.children().any(|child| child.is_element()) {
        return (PayloadStatus::ObjectNotText, None);
    }
    if facts.transforms != ["base64"] {
        return (PayloadStatus::TransformNotPlainBase64, None);
    }
    if !is_xml(&facts.mime) {
        return (PayloadStatus::NotXml, None);
    }
    let packed = packed_base64(object);
    match base64::decode(&packed) {
        Err(base64::Error::TooLarge) => (PayloadStatus::PayloadTooLarge, None),
        Err(base64::Error::Invalid) => (PayloadStatus::Base64Invalid, None),
        Ok(decoded) => {
            drop(packed);
            let outcome = form::inspect(&decoded, &Limits::default())
                .map(|inspection| inspection.form)
                .map_err(|failure| failure.code);
            (PayloadStatus::Inspected, Some(outcome))
        }
    }
}
