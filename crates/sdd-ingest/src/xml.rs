use knowledge::KnowledgeError;
use roxmltree::Node;

/// Text of the first direct child element with `name`, trimmed.
///
/// Returns `None` when the element is absent or holds only whitespace, so a
/// blank element is treated as missing rather than as an empty claim.
pub(crate) fn child_text<'a>(node: Node<'a, 'a>, name: &str) -> Option<&'a str> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
        .and_then(|child| child.text())
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// Required attribute value, trimmed.
pub(crate) fn require_attribute<'a>(
    node: Node<'a, 'a>,
    name: &str,
) -> Result<&'a str, KnowledgeError> {
    node.attribute(name)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            KnowledgeError::Parse(format!(
                "<{}> is missing the required '{name}' attribute",
                node.tag_name().name()
            ))
        })
}

/// First direct child element with `name`.
pub(crate) fn child_element<'a>(node: Node<'a, 'a>, name: &str) -> Option<Node<'a, 'a>> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
}
