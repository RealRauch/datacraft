//! Clean-room Jet/ACE binary layer (steps 2.2–2.4, items H5/H6).
//!
//! Public format documentation is the only source of knowledge here
//! (`docs/ARCHITECTURE.md` §12 — no MDB Tools source is read or quoted).

pub mod header;
pub mod page;

pub use header::{
    ACE_2007_VERSION, ACE_2010_VERSION, ACE_PRODUCT, FileHeader, JET_PRODUCT, JET4_VERSION,
    SIGNATURE,
};
pub use page::{JET3_PAGE_SIZE, next_page_pointer};
