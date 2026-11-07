//! Jet4 text index sort keys (step 49.2, item M63).
//!
//! The engine does not store plain characters in index leaves: text keys are
//! collation sort keys of the LANGID `0x0409`/CP1252 "General" sort order.
//! The grammar and the weight table are **observed engine output** (DAO oracle
//! probe batch, 2026-11-07) and documented in `docs/FORMAT-JET.md`
//! ("Fresh-create template and the engine's open gates"). DataCraft embeds the
//! observed table as a data constant; characters outside it answer a precise
//! `Unsupported` instead of being guessed.

#[cfg(test)]
mod tests;
