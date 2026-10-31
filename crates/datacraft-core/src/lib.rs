//! DataCraft core — headless domain model, command bus and application state.
//!
//! The GUI ([`datacraft-ui`](https://docs.rs/datacraft-ui)) and future
//! MCP/JSON-RPC endpoints are thin clients of this crate: every user action
//! maps to one command, every observable outcome is queryable as serializable
//! state. See `docs/ARCHITECTURE.md` §5.
//!
//! Test-first (step 1.2): the tests below define the object identity scheme
//! and stay RED until `ObjectKind`/`ObjectId` are implemented.

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn object_id_renders_deterministic_node_test_id() {
        let id = ObjectId::new(ObjectKind::Table, "Customers");
        assert_eq!(id.ui_test_id(), "node.table.customers");
    }

    proptest! {
        /// Any UTF-8 name survives a JSON round-trip unchanged.
        #[test]
        fn object_id_json_roundtrip_preserves_identity(name in ".*") {
            let id = ObjectId::new(ObjectKind::Query, name.as_str());
            let json = serde_json::to_string(&id).expect("serialize ObjectId");
            let back: ObjectId = serde_json::from_str(&json).expect("deserialize ObjectId");
            prop_assert_eq!(back, id);
        }
    }
}
