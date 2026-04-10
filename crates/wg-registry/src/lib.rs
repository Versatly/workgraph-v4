#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Runtime registry wrapper around the serializable [`wg_types::Registry`] data model.

use std::collections::BTreeMap;
use wg_error::{Result, WorkgraphError};
use wg_types::{PrimitiveType, Registry};

/// Runtime lookup structure for primitive type definitions.
#[derive(Debug, Clone, Default)]
pub struct RuntimeRegistry {
    types: BTreeMap<String, PrimitiveType>,
}

impl RuntimeRegistry {
    /// Creates an empty runtime registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a runtime registry with all built-in primitive types pre-registered.
    #[must_use]
    pub fn with_builtins() -> Self {
        Self::from_registry(Registry::builtins()).expect("builtin registry should be valid")
    }

    /// Creates a runtime registry from a serializable registry model.
    pub fn from_registry(registry: Registry) -> Result<Self> {
        let mut runtime_registry = Self::new();

        for primitive_type in registry.types {
            runtime_registry.register_type(primitive_type)?;
        }

        Ok(runtime_registry)
    }

    /// Registers a new primitive type definition.
    ///
    /// Returns a registry error when a type with the same name already exists.
    pub fn register_type(&mut self, primitive_type: PrimitiveType) -> Result<()> {
        let name = primitive_type.name.clone();

        if self.types.contains_key(&name) {
            return Err(WorkgraphError::RegistryError(format!(
                "primitive type '{name}' is already registered"
            )));
        }

        self.types.insert(name, primitive_type);
        Ok(())
    }

    /// Returns the primitive type definition with the given name, if it exists.
    #[must_use]
    pub fn get_type(&self, name: &str) -> Option<&PrimitiveType> {
        self.types.get(name)
    }

    /// Returns all registered primitive type definitions in sorted name order.
    #[must_use]
    pub fn list_types(&self) -> Vec<&PrimitiveType> {
        self.types.values().collect()
    }

    /// Converts the runtime registry back into its serializable data model.
    #[must_use]
    pub fn into_registry(self) -> Registry {
        Registry::new(self.types.into_values().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeRegistry;
    use wg_error::WorkgraphError;
    use wg_types::{FieldDefinition, PrimitiveType, Registry};

    fn custom_type(name: &str) -> PrimitiveType {
        PrimitiveType::new(
            name,
            format!("{name}s"),
            "Custom test type",
            vec![FieldDefinition::new(
                "id",
                "string",
                "Stable identifier",
                true,
                false,
            )],
        )
    }

    #[test]
    fn with_builtins_registers_all_required_types() {
        let registry = RuntimeRegistry::with_builtins();
        let listed = registry.list_types();

        assert_eq!(listed.len(), 18);
        assert_eq!(
            registry
                .get_type("person")
                .expect("person should be registered")
                .directory,
            "people"
        );
        assert!(registry.get_type("run").is_some());
        assert!(registry.get_type("trigger_receipt").is_some());
        assert!(registry.get_type("checkpoint").is_some());
        assert!(listed.windows(2).all(|pair| pair[0].name <= pair[1].name));
    }

    #[test]
    fn duplicate_registration_returns_registry_error() {
        let mut registry = RuntimeRegistry::new();
        let primitive_type = custom_type("decision");
        registry
            .register_type(primitive_type.clone())
            .expect("first registration should succeed");

        let error = registry
            .register_type(primitive_type)
            .expect_err("duplicate registration should fail");

        match error {
            WorkgraphError::RegistryError(message) => {
                assert!(message.contains("decision"));
                assert!(message.contains("already registered"));
            }
            other => panic!("expected registry error, got {other:?}"),
        }
    }

    #[test]
    fn from_registry_rejects_duplicate_entries() {
        let registry = Registry::new(vec![custom_type("custom"), custom_type("custom")]);
        let error = RuntimeRegistry::from_registry(registry)
            .expect_err("duplicate data model entries should fail");

        assert_eq!(error.code(), "registry_error");
    }

    #[test]
    fn custom_registration_is_visible_through_lookup_and_roundtrip() {
        let mut registry = RuntimeRegistry::new();
        registry
            .register_type(custom_type("workflow"))
            .expect("custom type should register");

        let serialized = registry.clone().into_registry();

        assert_eq!(registry.list_types().len(), 1);
        assert_eq!(
            registry
                .get_type("workflow")
                .expect("workflow type should exist")
                .directory,
            "workflows"
        );
        assert_eq!(
            serialized.get_type("workflow"),
            registry.get_type("workflow")
        );
    }
}
