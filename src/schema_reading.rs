use std::collections::BTreeSet;

use serde_json::Value;

use crate::Error;
use crate::openapi::Contract;

const MAX_REFERENCE_HOPS: usize = 32;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum SchemaReadStop {
    Cycle { pointer: String },
    DepthLimit { pointer: String },
}

#[derive(Debug)]
pub(crate) struct SchemaReadNode<'a> {
    pub schema: &'a Value,
    pub pointer: String,
    pub resource_uri: String,
    pub reference: Option<String>,
    pub siblings_apply: bool,
}

impl SchemaReadNode<'_> {
    /// Return source facts applicable at this node without merging source values.
    pub fn facts(&self) -> Option<Value> {
        let object = self.schema.as_object()?;
        if self.reference.is_none() {
            return Some(self.schema.clone());
        }
        if !self.siblings_apply {
            return None;
        }
        let siblings = object
            .iter()
            .filter(|(key, _)| !matches!(key.as_str(), "$ref" | "$dynamicRef"))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<serde_json::Map<_, _>>();
        (!siblings.is_empty()).then_some(Value::Object(siblings))
    }

    pub fn has_true(&self, keyword: &str) -> bool {
        self.facts()
            .and_then(|facts| facts.get(keyword).and_then(Value::as_bool))
            == Some(true)
    }
}

#[derive(Debug)]
pub(crate) struct SchemaReading<'a> {
    /// Ordered source nodes, including the start and any reached terminal node.
    pub nodes: Vec<SchemaReadNode<'a>>,
    pub terminal_index: Option<usize>,
    /// Resource scope in traversal order; lexical ancestors are not preloaded.
    pub scope: Vec<String>,
    pub stop: Option<SchemaReadStop>,
}

impl SchemaReading<'_> {
    pub fn terminal(&self) -> Option<&SchemaReadNode<'_>> {
        self.terminal_index.and_then(|index| self.nodes.get(index))
    }

    pub fn has_true(&self, keyword: &str) -> bool {
        self.nodes.iter().any(|node| node.has_true(keyword))
    }

    pub fn reference_facts(&self) -> impl Iterator<Item = (&str, Value)> {
        self.nodes.iter().filter_map(|node| {
            node.reference
                .as_ref()
                .and_then(|_| node.facts().map(|facts| (node.pointer.as_str(), facts)))
        })
    }
}

/// Follow a schema reference chain using the existing per-contract resolver.
///
/// Each borrowed source node and pointer remains available so callers can
/// interpret annotations without flattening or mutating schema values.
pub(crate) fn read_schema<'a>(
    contract: &'a Contract,
    mut schema: &'a Value,
    pointer: &str,
    inherited_scope: &[String],
) -> Result<SchemaReading<'a>, Error> {
    let mut current_pointer = pointer.to_owned();
    let first_resource = contract.schema_resources.resource_for_pointer(pointer);
    let mut scope = inherited_scope.to_vec();
    if !scope.contains(&first_resource) {
        scope.push(first_resource);
    }
    let mut visited = BTreeSet::new();
    let mut nodes = Vec::new();

    for _ in 0..MAX_REFERENCE_HOPS {
        let resource_uri = contract
            .schema_resources
            .resource_for_pointer(&current_pointer);
        let cycle_key = (current_pointer.clone(), scope.clone());
        if !visited.insert(cycle_key) {
            return Ok(SchemaReading {
                nodes,
                terminal_index: None,
                scope,
                stop: Some(SchemaReadStop::Cycle {
                    pointer: current_pointer,
                }),
            });
        }

        let reference = schema.as_object().and_then(|object| {
            object
                .get("$dynamicRef")
                .and_then(Value::as_str)
                .map(|reference| (reference.to_owned(), true))
                .or_else(|| {
                    object
                        .get("$ref")
                        .and_then(Value::as_str)
                        .map(|reference| (reference.to_owned(), false))
                })
        });
        let siblings_apply = reference.is_none() || contract.is_openapi_31;
        nodes.push(SchemaReadNode {
            schema,
            pointer: current_pointer.clone(),
            resource_uri,
            reference: reference.as_ref().map(|(reference, _)| reference.clone()),
            siblings_apply,
        });

        let Some((reference, dynamic)) = reference else {
            return Ok(SchemaReading {
                terminal_index: Some(nodes.len() - 1),
                nodes,
                scope,
                stop: None,
            });
        };

        let target =
            contract.resolve_schema_reference(&current_pointer, &reference, dynamic, &scope)?;
        schema = contract
            .schema_resources
            .schema_at(&contract.document, &target.pointer)
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "resolved schema target {} is missing",
                    target.pointer
                ))
            })?;
        current_pointer = target.pointer;
        if !scope.contains(&target.uri) {
            scope.push(target.uri);
        }
    }

    Ok(SchemaReading {
        nodes,
        terminal_index: None,
        scope,
        stop: Some(SchemaReadStop::DepthLimit {
            pointer: current_pointer,
        }),
    })
}
