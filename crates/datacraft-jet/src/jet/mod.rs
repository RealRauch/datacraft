//! Clean-room Jet/ACE binary layer (steps 2.2–2.4, items H5/H6).
//!
//! Public format documentation is the only source of knowledge here; the
//! durable fact record with sources lives in `docs/FORMAT-JET.md`.

pub mod access;
pub mod catalog;
pub mod collation;
pub mod complex;
pub mod create;
pub mod crypto;
pub mod data_page;
pub mod encode;
pub mod header;
pub mod index;
pub mod index_pages;
pub mod jet3;
pub mod lval;
pub mod page;
pub mod queries;
pub mod reader;
pub mod row;
pub mod tdef;
pub(crate) mod text;
pub mod usage_map;
pub mod vba;
pub mod writer;
pub mod writer_acl;
pub mod writer_drop;
pub mod writer_indexes;
pub mod writer_rows;
pub mod writer_schema;
pub mod writer_tables;

pub use access::{AccessStorage, AccessStorageEntry, OBJECT_TYPE_MACRO, StoredMacro};
pub use catalog::{CATALOG_TDEF_PAGE, CatalogEntry, OBJECT_TYPE_TABLE};
pub use data_page::DataPage;
pub use header::{
    ACE_2007_VERSION, ACE_2010_VERSION, ACE_PRODUCT, FileHeader, JET_PRODUCT, JET4_VERSION,
    SIGNATURE,
};
pub use page::{
    JET3_PAGE_SIZE, PAGE_TYPE_DATA, PAGE_TYPE_INDEX_LEAF, PAGE_TYPE_INTERMEDIATE_INDEX,
    PAGE_TYPE_TDEF, page_type,
};
pub use queries::{
    NativeJoin, NativeJoinKind, NativeOrder, NativeQuery, NativeQueryColumn, NativeQueryKind,
    OBJECT_TYPE_QUERY,
};
pub use reader::{Jet4TableReader, JetTableData};
pub use row::{decode_row, decode_row_jet3};
pub use tdef::{JetColumn, JetTableDef, JetType};
pub use vba::{StoredModuleKind, StoredVbaModule, StoredVbaProject};
pub use writer::Jet4Writer;
