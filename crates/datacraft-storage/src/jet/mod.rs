//! Clean-room Jet/ACE binary layer (steps 2.2–2.4, items H5/H6).
//!
//! Public format documentation is the only source of knowledge here; the
//! durable fact record with sources lives in `docs/FORMAT-JET.md`.

pub mod catalog;
pub mod data_page;
pub mod header;
pub mod lval;
pub mod page;
pub mod reader;
pub mod row;
pub mod tdef;
pub mod usage_map;
pub mod writer;

pub use catalog::{CATALOG_TDEF_PAGE, CatalogEntry, OBJECT_TYPE_TABLE};
pub use data_page::DataPage;
pub use header::{
    ACE_2007_VERSION, ACE_2010_VERSION, ACE_PRODUCT, FileHeader, JET_PRODUCT, JET4_VERSION,
    SIGNATURE,
};
pub use page::{JET3_PAGE_SIZE, PAGE_TYPE_DATA, PAGE_TYPE_TDEF, page_type};
pub use reader::{Jet4TableReader, JetTableData};
pub use row::decode_row;
pub use tdef::{JetColumn, JetTableDef, JetType};
pub use writer::Jet4Writer;
