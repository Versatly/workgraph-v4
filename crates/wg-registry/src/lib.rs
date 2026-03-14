//! Primitive type registry with built-in schema registration.

use std::collections::BTreeMap;

use wg_error::{Result, WorkgraphError};
use wg_types::{FieldDefinition, PrimitiveSchema, PrimitiveType};

/// In-memory schema registry for built-in and user-defined primitive types.
#[derive(Debug, Clone, Default)]
pub struct TypeRegistry {
    schemas: BTreeMap<PrimitiveType, PrimitiveSchema>,
}

impl TypeRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            schemas: BTreeMap::new(),
        }
    }

    /// Creates a registry pre-populated with all built-in primitive schemas.
    #[must_use]
    pub fn with_builtin_types() -> Self {
        let mut registry = Self::new();
        registry.register_builtin_types();
        registry
    }

    /// Registers all built-in primitive schemas.
    pub fn register_builtin_types(&mut self) {
        for schema in builtin_schemas() {
            self.schemas.insert(schema.primitive_type.clone(), schema);
        }
    }

    /// Registers a primitive schema, returning an error if it already exists.
    pub fn register_type(&mut self, schema: PrimitiveSchema) -> Result<()> {
        if self.schemas.contains_key(&schema.primitive_type) {
            return Err(WorkgraphError::DuplicateType(
                schema.primitive_type.as_str().to_owned(),
            ));
        }
        self.schemas.insert(schema.primitive_type.clone(), schema);
        Ok(())
    }

    /// Returns a schema by primitive type.
    #[must_use]
    pub fn get_type(&self, primitive_type: &PrimitiveType) -> Option<&PrimitiveSchema> {
        self.schemas.get(primitive_type)
    }

    /// Returns all registered schemas in deterministic order.
    #[must_use]
    pub fn list_types(&self) -> Vec<&PrimitiveSchema> {
        self.schemas.values().collect()
    }
}

fn builtin_schemas() -> Vec<PrimitiveSchema> {
    vec![
        schema(
            PrimitiveType::Decision,
            vec![
                required("status", "string"),
                required("decided_by", "string"),
            ],
        ),
        schema(PrimitiveType::Pattern, vec![required("steps", "string[]")]),
        schema(
            PrimitiveType::Lesson,
            vec![required("learned_from", "string")],
        ),
        schema(
            PrimitiveType::Policy,
            vec![required("enforcement", "string")],
        ),
        schema(
            PrimitiveType::Relationship,
            vec![required("source", "string"), required("target", "string")],
        ),
        schema(
            PrimitiveType::StrategicNote,
            vec![optional("horizon", "string")],
        ),
        schema(PrimitiveType::Org, vec![optional("mission", "string")]),
        schema(
            PrimitiveType::Team,
            vec![optional("responsibilities", "string[]")],
        ),
        schema(
            PrimitiveType::Person,
            vec![optional("preferences", "string[]")],
        ),
        schema(
            PrimitiveType::Agent,
            vec![optional("capabilities", "string[]")],
        ),
        schema(PrimitiveType::Client, vec![optional("status", "string")]),
        schema(PrimitiveType::Project, vec![optional("status", "string")]),
    ]
}

fn schema(primitive_type: PrimitiveType, fields: Vec<FieldDefinition>) -> PrimitiveSchema {
    PrimitiveSchema {
        primitive_type,
        fields,
    }
}

fn required(name: &str, field_type: &str) -> FieldDefinition {
    FieldDefinition {
        name: name.to_owned(),
        field_type: field_type.to_owned(),
        required: true,
        description: None,
    }
}

fn optional(name: &str, field_type: &str) -> FieldDefinition {
    FieldDefinition {
        name: name.to_owned(),
        field_type: field_type.to_owned(),
        required: false,
        description: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_registered() {
        let registry = TypeRegistry::with_builtin_types();
        assert_eq!(registry.list_types().len(), PrimitiveType::builtins().len());
        assert!(registry.get_type(&PrimitiveType::Org).is_some());
    }

    #[test]
    fn duplicate_registration_fails() {
        let mut registry = TypeRegistry::new();
        let schema = PrimitiveSchema {
            primitive_type: PrimitiveType::Client,
            fields: vec![],
        };
        registry
            .register_type(schema.clone())
            .expect("first registration should succeed");
        let error = registry
            .register_type(schema)
            .expect_err("second registration should fail");
        assert_eq!(error.code().as_str(), "conflict");
    }
}
