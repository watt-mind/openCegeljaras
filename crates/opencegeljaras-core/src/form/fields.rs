//! Field listing and field-level comparison over one bounded form tree.
//!
//! A field is a leaf element, an attribute, or a mixed element below the
//! variant root. Each one is addressed by a path of element local names with
//! positional indices wherever same-named siblings exist. Paths are never
//! materialised as owned joined strings while walking: a field keeps one index
//! into a component arena that borrows the document's own structure, so a deep
//! chain of long names over many leaves costs the arena, not the product of
//! depth and field count.
//!
//! Nothing here performs filesystem, network, clock, or process work, and a
//! field listing establishes neither schema conformance nor signature
//! validity, filing, or acceptance.

use std::collections::HashMap;

use roxmltree::{Document, Node};

use crate::form::tree::XSI_NAMESPACE;
use crate::form::{Inspection, NAME_LIMIT, truncate_name};
use crate::truncate_scalars;

/// Largest number of fields listed from one document.
pub const FIELD_LIMIT: usize = 10_000;
/// Largest number of Unicode scalar values emitted from one field value.
pub const VALUE_LIMIT: usize = 4096;
/// Largest number of Unicode scalar values emitted from one path.
pub const PATH_LIMIT: usize = 1024;
/// The fixed notice attached to every completed field listing.
pub const FIELDS_NOTICE: &str = "field listing of one recognised form; values are shown only on request; not validation, verification, or acceptance";
/// The fixed notice attached to every completed field comparison.
pub const DIFF_NOTICE: &str =
    "field-level comparison of two recognised forms; not validation, verification, or acceptance";

/// The parent marker used for a path component directly under the root.
const NO_PARENT: u32 = u32::MAX;

/// What kind of document node one field is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// An element without element children.
    Element,
    /// An attribute that is neither a namespace declaration nor in the XSI
    /// namespace.
    Attribute,
    /// An element with both element children and text that is not only XML
    /// whitespace. It has no value and its children are still walked.
    Mixed,
}

impl FieldKind {
    /// The stable machine-readable kind.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Element => "element",
            Self::Attribute => "attribute",
            Self::Mixed => "mixed",
        }
    }
}

/// One component of a path: a name, its positional index, and its parent.
///
/// Components form an arena in which every entry is one distinct path prefix,
/// so a field's whole identity is one `u32`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PathComponent {
    /// The component this one hangs under, or `None` at the variant root.
    parent: Option<u32>,
    /// The element or attribute local name, uncut.
    name: String,
    /// The 1-based document-order index, when same-named siblings exist.
    index: Option<u32>,
    /// Whether this component names an attribute rather than an element.
    attribute: bool,
}

/// One listed field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// The path component that identifies this field.
    component: u32,
    /// What kind of node it is.
    kind: FieldKind,
    /// An index into the listing's namespace table, or `None`.
    namespace: Option<u32>,
    /// The uncut value, absent for a mixed element.
    value: Option<String>,
}

impl Field {
    /// What kind of node this field is.
    #[must_use]
    pub fn kind(&self) -> FieldKind {
        self.kind
    }

    /// The value in Unicode scalar values, before any cut.
    #[must_use]
    pub fn value_length(&self) -> Option<usize> {
        self.value.as_ref().map(|value| value.chars().count())
    }

    /// Whether the emitted value is cut short.
    #[must_use]
    pub fn value_truncated(&self) -> Option<bool> {
        self.value_length().map(|length| length > VALUE_LIMIT)
    }

    /// The value as it is emitted, cut to [`VALUE_LIMIT`] scalars.
    #[must_use]
    pub fn emitted_value(&self) -> Option<String> {
        self.value
            .as_deref()
            .map(|value| truncate_scalars(value, VALUE_LIMIT))
    }
}

/// One rendered path and whether anything in it was cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    /// The path as it is emitted.
    pub text: String,
    /// Whether a component or the whole path was cut short.
    pub truncated: bool,
}

/// Every field of one recognised form, in emission order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldList {
    /// The path component arena. A parent always precedes its children.
    components: Vec<PathComponent>,
    /// The distinct namespace URIs the listed fields carry, uncut.
    namespaces: Vec<String>,
    /// The listed fields.
    fields: Vec<Field>,
    /// Whether the walk stopped at the field cap.
    truncated: bool,
}

impl FieldList {
    /// The listed fields, in emission order.
    #[must_use]
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// How many fields are listed. The total is unknown when truncated.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Whether no field at all was listed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Whether the walk stopped at the field cap.
    #[must_use]
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// The path of one listed field, cut as it is emitted.
    ///
    /// # Panics
    ///
    /// Panics when `index` is not a listed field.
    #[must_use]
    pub fn path(&self, index: usize) -> Path {
        let mut chain = Vec::new();
        let mut current = Some(self.fields[index].component);
        while let Some(id) = current {
            chain.push(id);
            current = self.components[id as usize].parent;
        }

        let mut text = String::new();
        let mut truncated = false;
        for (position, id) in chain.iter().rev().enumerate() {
            let component = &self.components[*id as usize];
            if position > 0 {
                text.push('/');
            }
            if component.attribute {
                text.push('@');
            }
            truncated |= component.name.chars().count() > NAME_LIMIT;
            text.push_str(&truncate_name(&component.name));
            if let Some(index) = component.index {
                text.push('[');
                text.push_str(&index.to_string());
                text.push(']');
            }
        }
        truncated |= text.chars().count() > PATH_LIMIT;
        Path {
            text: truncate_scalars(&text, PATH_LIMIT),
            truncated,
        }
    }

    /// The namespace of one listed field, cut as it is emitted.
    #[must_use]
    pub fn namespace(&self, index: usize) -> Option<String> {
        self.raw_namespace(index).map(truncate_name)
    }

    /// The uncut namespace of one listed field, for exact comparison.
    fn raw_namespace(&self, index: usize) -> Option<&str> {
        self.fields[index]
            .namespace
            .map(|id| self.namespaces[id as usize].as_str())
    }
}

/// Everything a field listing established about one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// The `form inspect` result for the same bytes.
    pub inspection: Inspection,
    /// The fields, when the document is a recognised form.
    pub fields: Option<FieldList>,
}

/// The field-level difference between two listings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    /// Indices of new fields present only in the new listing, in new order.
    pub added: Vec<usize>,
    /// Indices of fields present only in the old listing, in old order.
    pub removed: Vec<usize>,
    /// Pairs of old and new indices that differ, in new order.
    pub changed: Vec<(usize, usize)>,
    /// How many paths carry the same kind, namespace, and value on both sides.
    pub unchanged: usize,
}

/// Whether an attribute is a field rather than an unreported hint.
///
/// Namespace declarations are not attributes in the parsed tree at all, and
/// XSI attributes name schema locations and nil-ness, which this crate reports
/// as structure elsewhere and never resolves.
fn is_field_attribute(attribute: &roxmltree::Attribute<'_, '_>) -> bool {
    attribute.namespace() != Some(XSI_NAMESPACE)
}

/// Whether one text slice is only XML whitespace.
fn is_xml_whitespace(text: &str) -> bool {
    text.chars()
        .all(|character| matches!(character, ' ' | '\t' | '\r' | '\n'))
}

/// The walk state: the arena, the namespace table, and the listed fields.
struct Walker {
    list: FieldList,
}

impl Walker {
    fn new() -> Self {
        Self {
            list: FieldList {
                components: Vec::new(),
                namespaces: Vec::new(),
                fields: Vec::new(),
                truncated: false,
            },
        }
    }

    /// Record one path component and return its arena index.
    fn component(
        &mut self,
        parent: Option<u32>,
        name: &str,
        index: Option<u32>,
        attribute: bool,
    ) -> u32 {
        let id = u32::try_from(self.list.components.len()).unwrap_or(NO_PARENT);
        self.list.components.push(PathComponent {
            parent,
            name: name.to_owned(),
            index,
            attribute,
        });
        id
    }

    /// Intern one namespace URI, returning its table index.
    fn namespace(&mut self, namespace: Option<&str>) -> Option<u32> {
        let namespace = namespace?;
        let existing = self
            .list
            .namespaces
            .iter()
            .position(|known| known == namespace);
        let position = existing.unwrap_or_else(|| {
            self.list.namespaces.push(namespace.to_owned());
            self.list.namespaces.len() - 1
        });
        u32::try_from(position).ok()
    }

    /// List one field, or record that the cap stopped the walk.
    fn emit(
        &mut self,
        component: u32,
        kind: FieldKind,
        namespace: Option<&str>,
        value: Option<String>,
    ) -> bool {
        if self.list.fields.len() >= FIELD_LIMIT {
            self.list.truncated = true;
            return false;
        }
        let namespace = self.namespace(namespace);
        self.list.fields.push(Field {
            component,
            kind,
            namespace,
            value,
        });
        true
    }
}

/// The value of a leaf element: every text child, in document order.
fn leaf_value(node: Node<'_, '_>) -> String {
    let mut value = String::new();
    for child in node.children() {
        if let Some(text) = child.text().filter(|_| child.is_text()) {
            value.push_str(text);
        }
    }
    value
}

/// How one element is listed, if at all.
fn element_kind(node: Node<'_, '_>) -> Option<FieldKind> {
    if !node.children().any(|child| child.is_element()) {
        return Some(FieldKind::Element);
    }
    let mixed = node.children().any(|child| {
        child
            .text()
            .filter(|_| child.is_text())
            .is_some_and(|text| !is_xml_whitespace(text))
    });
    mixed.then_some(FieldKind::Mixed)
}

/// The 1-based index of every element child, counting siblings once per
/// parent.
///
/// Same-named means the same local name, whatever the namespace, so an element
/// with a same-named sibling in another namespace is indexed too.
fn child_indices<'a, 'input>(parent: Node<'a, 'input>) -> Vec<(Node<'a, 'input>, Option<u32>)> {
    let mut totals: HashMap<&str, u32> = HashMap::new();
    for child in parent.children().filter(Node::is_element) {
        *totals.entry(child.tag_name().name()).or_default() += 1;
    }
    let mut seen: HashMap<&str, u32> = HashMap::new();
    let mut indexed = Vec::new();
    for child in parent.children().filter(Node::is_element) {
        let name = child.tag_name().name();
        let position = seen.entry(name).or_default();
        *position += 1;
        let index = (totals.get(name).copied().unwrap_or(1) > 1).then_some(*position);
        indexed.push((child, index));
    }
    indexed
}

/// The 1-based index of every listed attribute of one element.
fn attribute_indices<'a, 'input>(
    element: Node<'a, 'input>,
) -> Vec<(roxmltree::Attribute<'a, 'input>, Option<u32>)> {
    let mut totals: HashMap<&str, u32> = HashMap::new();
    for attribute in element.attributes().filter(is_field_attribute) {
        *totals.entry(attribute.name()).or_default() += 1;
    }
    let mut seen: HashMap<&str, u32> = HashMap::new();
    let mut indexed = Vec::new();
    for attribute in element.attributes().filter(is_field_attribute) {
        let name = attribute.name();
        let position = seen.entry(name).or_default();
        *position += 1;
        let index = (totals.get(name).copied().unwrap_or(1) > 1).then_some(*position);
        indexed.push((attribute, index));
    }
    indexed
}

/// Walk one recognised document's variant root and list its fields.
pub(crate) fn walk(document: &Document<'_>) -> FieldList {
    let mut walker = Walker::new();
    let Some(root) = document
        .root_element()
        .children()
        .find(roxmltree::Node::is_element)
    else {
        return walker.list;
    };

    let mut stack = vec![(root, None)];
    while let Some((node, component)) = stack.pop() {
        if !visit(&mut walker, node, component) {
            break;
        }
        let mut children = Vec::new();
        for (child, index) in child_indices(node) {
            let id = walker.component(component, child.tag_name().name(), index, false);
            children.push((child, Some(id)));
        }
        stack.extend(children.into_iter().rev());
    }
    walker.list
}

/// List one element's own entry and its attributes. `false` stops the walk.
///
/// The variant root itself is not a field, so it contributes only attributes.
fn visit(walker: &mut Walker, node: Node<'_, '_>, component: Option<u32>) -> bool {
    if let Some(id) = component {
        match element_kind(node) {
            None => {}
            Some(FieldKind::Mixed) => {
                if !walker.emit(id, FieldKind::Mixed, node.tag_name().namespace(), None) {
                    return false;
                }
            }
            Some(kind) => {
                let value = leaf_value(node);
                if !walker.emit(id, kind, node.tag_name().namespace(), Some(value)) {
                    return false;
                }
            }
        }
    }

    for (attribute, index) in attribute_indices(node) {
        let id = walker.component(component, attribute.name(), index, true);
        if !walker.emit(
            id,
            FieldKind::Attribute,
            attribute.namespace(),
            Some(attribute.value().to_owned()),
        ) {
            return false;
        }
    }
    true
}

/// Compare two field listings by their untruncated paths.
///
/// Paths are compared component by component through the two arenas, so no
/// joined path string is ever built for the comparison. A component of the new
/// listing resolves to at most one component of the old listing, and the
/// resolution of a parent is always known before its children because a parent
/// always precedes them in the arena.
#[must_use]
pub fn diff(old: &FieldList, new: &FieldList) -> Diff {
    let mut known: HashMap<(u32, &str, Option<u32>, bool), u32> =
        HashMap::with_capacity(old.components.len());
    for (id, component) in old.components.iter().enumerate() {
        let key = (
            component.parent.unwrap_or(NO_PARENT),
            component.name.as_str(),
            component.index,
            component.attribute,
        );
        known.insert(key, u32::try_from(id).unwrap_or(NO_PARENT));
    }

    let mut resolved: Vec<Option<u32>> = Vec::with_capacity(new.components.len());
    for (id, component) in new.components.iter().enumerate() {
        let parent = match component.parent {
            None => Some(NO_PARENT),
            Some(parent) => {
                debug_assert!((parent as usize) < id, "a parent precedes its children");
                resolved.get(parent as usize).copied().flatten()
            }
        };
        let found = parent.and_then(|parent| {
            known
                .get(&(
                    parent,
                    component.name.as_str(),
                    component.index,
                    component.attribute,
                ))
                .copied()
        });
        resolved.push(found);
    }

    let mut field_at = vec![usize::MAX; old.components.len()];
    for (index, field) in old.fields.iter().enumerate() {
        field_at[field.component as usize] = index;
    }
    compare(old, new, &resolved, &field_at)
}

/// Classify every new field, then collect the old fields nothing matched.
fn compare(old: &FieldList, new: &FieldList, resolved: &[Option<u32>], field_at: &[usize]) -> Diff {
    let mut diff = Diff {
        added: Vec::new(),
        removed: Vec::new(),
        changed: Vec::new(),
        unchanged: 0,
    };
    let mut matched = vec![false; old.fields.len()];
    for (index, field) in new.fields.iter().enumerate() {
        let counterpart = resolved
            .get(field.component as usize)
            .copied()
            .flatten()
            .map(|component| field_at[component as usize])
            .filter(|found| *found != usize::MAX);
        let Some(counterpart) = counterpart else {
            diff.added.push(index);
            continue;
        };
        matched[counterpart] = true;
        if old.fields[counterpart].kind == field.kind
            && old.raw_namespace(counterpart) == new.raw_namespace(index)
            && old.fields[counterpart].value == field.value
        {
            diff.unchanged += 1;
        } else {
            diff.changed.push((counterpart, index));
        }
    }
    for (index, hit) in matched.iter().enumerate() {
        if !hit {
            diff.removed.push(index);
        }
    }
    diff
}

#[cfg(test)]
mod tests;
