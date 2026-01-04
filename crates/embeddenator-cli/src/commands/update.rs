//! Update command implementations (add, remove, modify, compact)
//!
//! NOTE: These operations require methods to be implemented in embeddenator-fs component.
//! Currently, they return errors indicating the features are not yet available.

use anyhow::Result;
use std::path::PathBuf;

pub fn handle_update_add(
    _engram: PathBuf,
    _manifest: PathBuf,
    _file: PathBuf,
    _logical_path: Option<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!(
            "Embeddenator v{} - Incremental Add",
            env!("CARGO_PKG_VERSION")
        );
        println!("===================================");
    }

    // TODO: add_file method needs to be implemented in embeddenator-fs
    // For now, return an error indicating this feature is not yet available
    anyhow::bail!(
        "Incremental add operation not yet implemented in embeddenator-fs component.\n\
        This feature requires the add_file() method to be added to EmbrFS.\n\
        Use full re-ingestion as a workaround."
    )
}

pub fn handle_update_remove(
    _engram: PathBuf,
    _manifest: PathBuf,
    _path: String,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!(
            "Embeddenator v{} - Incremental Remove",
            env!("CARGO_PKG_VERSION")
        );
        println!("======================================");
    }

    // TODO: remove_file method needs to be implemented in embeddenator-fs
    anyhow::bail!(
        "Incremental remove operation not yet implemented in embeddenator-fs component.\n\
        This feature requires the remove_file() method to be added to EmbrFS.\n\
        Use full re-ingestion as a workaround."
    )
}

pub fn handle_update_modify(
    _engram: PathBuf,
    _manifest: PathBuf,
    _file: PathBuf,
    _logical_path: Option<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!(
            "Embeddenator v{} - Incremental Modify",
            env!("CARGO_PKG_VERSION")
        );
        println!("======================================");
    }

    // TODO: modify_file method needs to be implemented in embeddenator-fs
    anyhow::bail!(
        "Incremental modify operation not yet implemented in embeddenator-fs component.\n\
        This feature requires the modify_file() method to be added to EmbrFS.\n\
        Use full re-ingestion as a workaround."
    )
}

pub fn handle_update_compact(
    _engram: PathBuf,
    _manifest: PathBuf,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!(
            "Embeddenator v{} - Compact Engram",
            env!("CARGO_PKG_VERSION")
        );
        println!("===================================");
    }

    // TODO: compact method needs to be implemented in embeddenator-fs
    anyhow::bail!(
        "Compact operation not yet implemented in embeddenator-fs component.\n\
        This feature requires the compact() method to be added to EmbrFS.\n\
        Use full re-ingestion as a workaround."
    )
}
