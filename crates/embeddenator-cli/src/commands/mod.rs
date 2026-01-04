//! Command implementations for CLI operations

pub mod ingest;
pub mod extract;
pub mod query;
pub mod bundle_hier;
pub mod mount;
pub mod update;

pub use ingest::handle_ingest;
pub use extract::handle_extract;
pub use query::{handle_query, handle_query_text};
pub use bundle_hier::handle_bundle_hier;
#[cfg(feature = "fuse")]
pub use mount::handle_mount;
pub use update::{handle_update_add, handle_update_remove, handle_update_modify, handle_update_compact};
