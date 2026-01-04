//! FUSE mount command implementation

#[cfg(feature = "fuse")]
use anyhow::Result;
#[cfg(feature = "fuse")]
use embeddenator_fs::embrfs::{EmbrFS, DEFAULT_CHUNK_SIZE};
#[cfg(feature = "fuse")]
use embeddenator_fs::fuse_shim::{EngramFS, MountOptions, mount};
#[cfg(feature = "fuse")]
use embeddenator_vsa::ReversibleVSAConfig;
#[cfg(feature = "fuse")]
use std::path::PathBuf;

#[cfg(feature = "fuse")]
pub fn handle_mount(
    engram: PathBuf,
    manifest: PathBuf,
    mountpoint: PathBuf,
    allow_other: bool,
    _foreground: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!(
            "Embeddenator v{} - FUSE Mount",
            env!("CARGO_PKG_VERSION")
        );
        println!("============================");
    }

    // Load engram and manifest
    let engram_data = EmbrFS::load_engram(&engram)?;
    let manifest_data = EmbrFS::load_manifest(&manifest)?;
    let config = ReversibleVSAConfig::default();

    if verbose {
        println!("Loaded engram: {}", engram.display());
        println!("Loaded manifest: {} files", manifest_data.files.len());
    }

    // Create FUSE filesystem and populate with decoded files
    let fuse_fs = EngramFS::new(true);

    for file_entry in &manifest_data.files {
        // Decode file data using the same approach as EmbrFS::extract
        let mut reconstructed = Vec::new();

        for &chunk_id in &file_entry.chunks {
            if let Some(chunk_vec) = engram_data.codebook.get(&chunk_id) {
                // Decode the sparse vector to bytes
                // IMPORTANT: Use the same path as during encoding for correct shift calculation
                let decoded = chunk_vec.decode_data(
                    &config,
                    Some(&file_entry.path),
                    DEFAULT_CHUNK_SIZE,
                );

                // Apply correction to guarantee bit-perfect reconstruction
                let chunk_data = if let Some(corrected) =
                    engram_data.corrections.apply(chunk_id as u64, &decoded)
                {
                    corrected
                } else {
                    // No correction found - use decoded directly
                    decoded
                };

                reconstructed.extend_from_slice(&chunk_data);
            }
        }

        // Truncate to exact file size
        reconstructed.truncate(file_entry.size);

        // Add to FUSE filesystem
        if let Err(e) = fuse_fs.add_file(&file_entry.path, reconstructed) {
            if verbose {
                eprintln!("Warning: Failed to add {}: {}", file_entry.path, e);
            }
        }
    }

    if verbose {
        println!(
            "Populated {} files into FUSE filesystem",
            fuse_fs.file_count()
        );
        println!("Total size: {} bytes", fuse_fs.total_size());
        println!("Mounting at: {}", mountpoint.display());
        println!();
    }

    // Verify mountpoint exists
    if !mountpoint.exists() {
        anyhow::bail!("Mountpoint does not exist: {}", mountpoint.display());
    }

    // Configure mount options
    let options = MountOptions {
        read_only: true,
        allow_other,
        allow_root: !allow_other,
        fsname: format!("engram:{}", engram.display()),
    };

    // Mount the filesystem (blocks until unmounted)
    println!("EngramFS mounted at {}", mountpoint.display());
    println!(
        "Use 'fusermount -u {}' to unmount",
        mountpoint.display()
    );

    mount(fuse_fs, &mountpoint, options)?;

    if verbose {
        println!("\nUnmounted.");
    }

    Ok(())
}

#[cfg(not(feature = "fuse"))]
pub fn handle_mount(
    _engram: std::path::PathBuf,
    _manifest: std::path::PathBuf,
    _mountpoint: std::path::PathBuf,
    _allow_other: bool,
    _foreground: bool,
    _verbose: bool,
) -> anyhow::Result<()> {
    anyhow::bail!("FUSE support not enabled. Build with --features fuse")
}
