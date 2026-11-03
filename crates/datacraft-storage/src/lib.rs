//! DataCraft storage composite (ADR-7, crate split step 31.2 / M19): the
//! format-detecting driver registry over the shipped backends plus the
//! deterministic sample generator. Out-of-tree driver crates depend only on
//! `datacraft-core`; this crate is the convenience composition for the
//! shipped binaries — it names the concrete drivers so the composition
//! roots (`datacraft`, `datacraft-cli`, `datacraft-mcp`) do not have to.

pub mod detecting;
pub mod samples;
