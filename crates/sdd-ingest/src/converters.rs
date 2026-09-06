use crate::{child_element, child_text};
use knowledge::KnowledgeError;
use std::collections::BTreeMap;

/// How a converter turns raw counts into a presented value.
///
/// The kind, the output unit and the arithmetic SDD declares — a linear
/// multiplier and offset, or a map's breakpoints — are retained verbatim as
/// text, so that a value can be presented the way SDD presents it and the
/// numbers can be traced back to the source unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConverterKind {
    Linear,
    Map,
}

impl ConverterKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Map => "map",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConverterInfo {
    pub id: String,
    pub kind: ConverterKind,
    pub out_quantity: Option<String>,
    pub out_unit: Option<String>,
    /// Linear converters: `multiplier` and `offset` attributes, verbatim.
    pub multiplier: Option<String>,
    pub offset: Option<String>,
    /// Whether the offset is applied before the multiplier (`offsetFirst`).
    pub offset_first: Option<bool>,
    /// Map converters: `(x, y)` breakpoints, verbatim, in document order.
    pub map_points: Vec<(String, String)>,
    /// Named raw-count ranges (`quantityState`), verbatim, in document order:
    /// what SDD shows instead of a number when the counts fall in the range.
    pub states: Vec<ConverterState>,
}

/// One `quantityState`: an inclusive raw-count range and the name SDD shows
/// for it. The bounds are kept as the source writes them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConverterState {
    pub low: String,
    pub high: String,
    pub name: String,
}

/// Lookup from an SDD converter id to its declared output quantity and unit.
///
/// A DID formatting file references converters by id through `SHORTCUT`, so the
/// catalogue must be loaded before formatting files are parsed. A missing
/// converter is not fatal: the parameter is still recorded, without a unit,
/// rather than being dropped or given an invented one.
#[derive(Clone, Debug, Default)]
pub struct ConverterCatalogue {
    entries: BTreeMap<String, ConverterInfo>,
}

impl ConverterCatalogue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, id: &str) -> Option<&ConverterInfo> {
        self.entries.get(id)
    }

    /// Parse one converter document and add it to the catalogue.
    ///
    /// Repeating an identical definition is accepted; a genuine redefinition is
    /// rejected rather than silently overwriting the first one.
    pub fn insert_from_xml(&mut self, input: &str) -> Result<&ConverterInfo, KnowledgeError> {
        let document = roxmltree::Document::parse(input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let component = document.root_element();
        if component.tag_name().name() != "COMPONENT" {
            return Err(KnowledgeError::Parse(format!(
                "expected a <COMPONENT> root, found <{}>",
                component.tag_name().name()
            )));
        }

        let converter = component
            .children()
            .find(|node| {
                node.is_element()
                    && matches!(node.tag_name().name(), "LinearConverter" | "MapConverter")
            })
            .ok_or_else(|| {
                KnowledgeError::Parse("no LinearConverter or MapConverter element".into())
            })?;
        let kind = match converter.tag_name().name() {
            "LinearConverter" => ConverterKind::Linear,
            _ => ConverterKind::Map,
        };
        let id = converter
            .attribute("id")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| KnowledgeError::Parse("converter is missing an id".into()))?
            .to_string();

        let out_type = child_element(converter, "Properties")
            .and_then(|properties| child_element(properties, "outType"));
        let attribute = |name: &str| {
            converter
                .attribute(name)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        };
        let map_points = converter
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "MAP_BREAK_POINT")
            .filter_map(|node| {
                Some((
                    node.attribute("x")?.trim().to_string(),
                    node.attribute("y")?.trim().to_string(),
                ))
            })
            .collect();
        let states = converter
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "quantityState")
            .filter_map(|state| {
                let properties = child_element(state, "Properties")?;
                let quantity = child_element(properties, "range")
                    .and_then(|range| child_element(range, "quantity"))?;
                let name = child_text(properties, "name")?;
                let name = name.trim();
                if name.is_empty() {
                    return None;
                }
                Some(ConverterState {
                    low: quantity.attribute("lowValue")?.trim().to_string(),
                    high: quantity.attribute("highValue")?.trim().to_string(),
                    name: name.to_string(),
                })
            })
            .collect();
        let info = ConverterInfo {
            id: id.clone(),
            kind,
            out_quantity: out_type
                .and_then(|node| node.attribute("quantity"))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            out_unit: out_type
                .and_then(|node| node.attribute("unit"))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            multiplier: attribute("multiplier"),
            offset: attribute("offset"),
            offset_first: attribute("offsetFirst").map(|value| value == "true"),
            map_points,
            states,
        };

        match self.entries.get(&id) {
            Some(existing) if existing == &info => {}
            Some(_) => {
                return Err(KnowledgeError::Parse(format!(
                    "conflicting definitions for converter {id}"
                )))
            }
            None => {
                self.entries.insert(id.clone(), info);
            }
        }
        Ok(self.entries.get(&id).expect("just inserted"))
    }
}
