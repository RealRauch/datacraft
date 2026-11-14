//! DataCraft query layer — QBE model, criteria parsing, evaluation.
//!
//! This crate holds the query-facing half of the engine: the QBE document
//! model (serde), the typed criteria parser, the QBE evaluation, native
//! saved-query execution and the native-query port vocabulary.
//!
//! It depends only on `datacraft-expr` and `datacraft-types`; the command
//! bus re-exports these modules, so clients keep addressing
//! `datacraft_core::{qbe, query_engine, native_exec, query_criteria}` and
//! the native-query port paths (phase 76, item L37).

pub mod native_exec;
pub mod native_query;
pub mod qbe;
pub mod query_criteria;
pub mod query_engine;
