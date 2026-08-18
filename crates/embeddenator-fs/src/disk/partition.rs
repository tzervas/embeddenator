//! Partition Table Detection and Parsing
//!
//! Disk images typically contain partition tables that divide the disk into
//! logical regions. This module handles GPT and MBR partition schemes.
//!
//! # Why Partition Table Support?
//!
//! Virtual machine disk images are almost always partitioned:
//! - **Boot partition**: Contains kernel, initramfs, bootloader
//! - **Root partition**: Main filesystem (/, /usr, /var, etc.)
//! - **Swap partition**: Virtual memory (often present but not encoded)
//! - **EFI System Partition**: UEFI boot files
//!
//! To encode a VM image, we need to:
//! 1. Detect the partition scheme (GPT vs MBR)
//! 2. Enumerate partitions
//! 3. Identify filesystem types
//! 4. Traverse each filesystem for encoding
//!
//! # GPT vs MBR
//!
//! ## GPT (GUID Partition Table)
//!
//! Modern partition scheme used by UEFI systems:
//! - Supports disks > 2TB
//! - Up to 128 partitions (no extended partitions needed)
//! - Redundant backup at end of disk
//! - Uses GUIDs for partition type identification
//!
//! ## MBR (Master Boot Record)
//!
//! Legacy partition scheme:
//! - Limited to 2TB disks (512-byte sectors)
//! - 4 primary partitions max (extended for more)
//! - Single byte partition type codes
//! - Still common in older VMs and embedded systems
//!
//! # Protective MBR
//!
//! GPT disks include a "protective MBR" that prevents old tools from
//! seeing the disk as unpartitioned and accidentally overwriting it.
//! We detect this and read the actual GPT instead.
//!
//! # Usage
//!
//! ```rust,ignore
//! use embeddenator_fs::disk::{RawImage, PartitionTable};
//!
//! let image = RawImage::open("disk.img").await?;
//! let table = PartitionTable::detect(&image).await?;
//!
//! for partition in table.partitions() {
//!     println!("{}: {} bytes at offset {}",
//!         partition.name,
//!         partition.size,
//!         partition.start_offset
//!     );
//! }
//! ```

use super::error::{DiskError, DiskResult};
use super::BlockDevice;

/// Partition type identifiers
///
/// # Why Track Partition Types?
///
/// Different partition types require different handling:
/// - Linux filesystems: Traverse and encode
/// - Swap: Skip (no persistent data)
/// - EFI: Encode with special handling for boot files
/// - Unknown: Treat as raw data or skip
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionType {
    /// Linux filesystem (ext2/3/4, XFS, Btrfs, etc.)
    LinuxFilesystem,

    /// Linux swap partition
    LinuxSwap,

    /// EFI System Partition (FAT32 with boot files)
    EfiSystem,

    /// Linux LVM Physical Volume
    LinuxLvm,

    /// Linux RAID member
    LinuxRaid,

    /// Microsoft Basic Data (NTFS, FAT32)
    MicrosoftBasicData,

    /// Unknown or unrecognized partition type
    Unknown(u8),
}

impl PartitionType {
    /// Create from GPT partition type GUID
    ///
    /// # Why GUIDs?
    ///
    /// GPT uses 128-bit GUIDs to identify partition types. This is more
    /// robust than MBR's single-byte codes and allows vendor-specific types.
    pub fn from_gpt_guid(guid: &[u8; 16]) -> Self {
        // GUIDs are stored in mixed-endian format in GPT
        // Compare against well-known GUIDs

        // Linux filesystem: 0FC63DAF-8483-4772-8E79-3D69D8477DE4
        const LINUX_FS: [u8; 16] = [
            0xAF, 0x3D, 0xC6, 0x0F, 0x83, 0x84, 0x72, 0x47, 0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47,
            0x7D, 0xE4,
        ];

        // Linux swap: 0657FD6D-A4AB-43C4-84E5-0933C84B4F4F
        const LINUX_SWAP: [u8; 16] = [
            0x6D, 0xFD, 0x57, 0x06, 0xAB, 0xA4, 0xC4, 0x43, 0x84, 0xE5, 0x09, 0x33, 0xC8, 0x4B,
            0x4F, 0x4F,
        ];

        // EFI System: C12A7328-F81F-11D2-BA4B-00A0C93EC93B
        const EFI_SYSTEM: [u8; 16] = [
            0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11, 0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E,
            0xC9, 0x3B,
        ];

        // Microsoft Basic Data: EBD0A0A2-B9E5-4433-87C0-68B6B72699C7
        const MS_BASIC_DATA: [u8; 16] = [
            0xA2, 0xA0, 0xD0, 0xEB, 0xE5, 0xB9, 0x33, 0x44, 0x87, 0xC0, 0x68, 0xB6, 0xB7, 0x26,
            0x99, 0xC7,
        ];

        if guid == &LINUX_FS {
            Self::LinuxFilesystem
        } else if guid == &LINUX_SWAP {
            Self::LinuxSwap
        } else if guid == &EFI_SYSTEM {
            Self::EfiSystem
        } else if guid == &MS_BASIC_DATA {
            Self::MicrosoftBasicData
        } else {
            Self::Unknown(0)
        }
    }

    /// Create from MBR partition type byte
    ///
    /// # Why Single Byte?
    ///
    /// MBR predates the need for extensive partition types. The single byte
    /// was sufficient for the limited number of operating systems in the 1980s.
    pub fn from_mbr_type(type_byte: u8) -> Self {
        match type_byte {
            0x83 => Self::LinuxFilesystem,
            0x82 => Self::LinuxSwap,
            0xEF => Self::EfiSystem,
            0x8E => Self::LinuxLvm,
            0xFD => Self::LinuxRaid,
            0x07 | 0x0B | 0x0C => Self::MicrosoftBasicData, // NTFS, FAT32
            _ => Self::Unknown(type_byte),
        }
    }

    /// Check if this partition type should be encoded
    ///
    /// # Why Skip Some Partitions?
    ///
    /// - Swap partitions contain no persistent data
    /// - Some partition types are just markers
    /// - Encoding everything wastes space and time
    pub fn should_encode(&self) -> bool {
        match self {
            Self::LinuxSwap => false, // No persistent data
            Self::LinuxLvm => false,  // Needs LVM metadata parsing (future)
            Self::LinuxRaid => false, // Needs RAID metadata parsing (future)
            _ => true,
        }
    }
}

/// Information about a single partition
#[derive(Debug, Clone)]
pub struct PartitionInfo {
    /// Partition number (0-indexed)
    pub number: u32,

    /// Human-readable name (from GPT or generated for MBR)
    pub name: String,

    /// Partition type
    pub partition_type: PartitionType,

    /// Start offset in bytes from beginning of disk
    pub start_offset: u64,

    /// Size in bytes
    pub size: u64,

    /// Unique identifier (GPT GUID or generated for MBR)
    pub uuid: Option<String>,

    /// Filesystem type hint (from GPT attributes or MBR type)
    pub fs_hint: Option<String>,
}

impl PartitionInfo {
    /// Get the end offset (exclusive)
    pub fn end_offset(&self) -> u64 {
        self.start_offset + self.size
    }
}

/// Partition table parsed from a disk image
#[derive(Debug)]
pub struct PartitionTable {
    /// Partition scheme type
    pub scheme: PartitionScheme,

    /// List of partitions
    partitions: Vec<PartitionInfo>,

    /// Disk size in bytes
    pub disk_size: u64,

    /// Logical block size
    pub block_size: u32,
}

/// Partition scheme type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionScheme {
    /// GUID Partition Table (modern)
    Gpt,
    /// Master Boot Record (legacy)
    Mbr,
    /// No partition table detected (whole-disk filesystem or raw data)
    None,
}

impl PartitionTable {
    /// Detect and parse partition table from a block device
    ///
    /// # Detection Order
    ///
    /// 1. Check for GPT at LBA 1 (offset 512)
    /// 2. If GPT found, use it (even if protective MBR exists)
    /// 3. If no GPT, check for MBR at LBA 0
    /// 4. If no MBR, assume unpartitioned disk
    ///
    /// # Why This Order?
    ///
    /// GPT disks always have a protective MBR for compatibility. If we
    /// checked MBR first, we'd incorrectly parse GPT disks as having
    /// a single partition covering the whole disk.
    pub async fn detect(device: &dyn BlockDevice) -> DiskResult<Self> {
        let disk_size = device.size();
        let block_size = device.block_size();

        // Try GPT first (at LBA 1)
        if let Ok(gpt) = Self::try_parse_gpt(device).await {
            return Ok(gpt);
        }

        // Fall back to MBR
        if let Ok(mbr) = Self::try_parse_mbr(device).await {
            return Ok(mbr);
        }

        // No partition table - treat as unpartitioned
        Ok(Self {
            scheme: PartitionScheme::None,
            partitions: vec![],
            disk_size,
            block_size,
        })
    }

    /// Try to parse a GPT partition table
    async fn try_parse_gpt(device: &dyn BlockDevice) -> DiskResult<Self> {
        // GPT header is at LBA 1 (offset 512)
        let mut header = [0u8; 92];
        device.read_at(&mut header, 512).await?;

        // Check GPT signature: "EFI PART"
        if &header[0..8] != b"EFI PART" {
            return Err(DiskError::PartitionError {
                reason: "No GPT signature found".to_string(),
            });
        }

        // Parse header fields
        let revision = u32::from_le_bytes(header[8..12].try_into().unwrap());
        let header_size = u32::from_le_bytes(header[12..16].try_into().unwrap());
        let _header_crc = u32::from_le_bytes(header[16..20].try_into().unwrap());
        let _current_lba = u64::from_le_bytes(header[24..32].try_into().unwrap());
        let _backup_lba = u64::from_le_bytes(header[32..40].try_into().unwrap());
        let first_usable_lba = u64::from_le_bytes(header[40..48].try_into().unwrap());
        let last_usable_lba = u64::from_le_bytes(header[48..56].try_into().unwrap());
        let partition_entry_lba = u64::from_le_bytes(header[72..80].try_into().unwrap());
        let num_partition_entries = u32::from_le_bytes(header[80..84].try_into().unwrap());
        let partition_entry_size = u32::from_le_bytes(header[84..88].try_into().unwrap());

        // Read partition entries
        let mut partitions = Vec::new();
        let entries_offset = partition_entry_lba * 512;

        for i in 0..num_partition_entries {
            let entry_offset = entries_offset + (i as u64 * partition_entry_size as u64);
            let mut entry = vec![0u8; partition_entry_size as usize];
            device.read_at(&mut entry, entry_offset).await?;

            // Check if partition is used (type GUID not all zeros)
            let type_guid: [u8; 16] = entry[0..16].try_into().unwrap();
            if type_guid == [0u8; 16] {
                continue; // Unused entry
            }

            let partition_guid: [u8; 16] = entry[16..32].try_into().unwrap();
            let start_lba = u64::from_le_bytes(entry[32..40].try_into().unwrap());
            let end_lba = u64::from_le_bytes(entry[40..48].try_into().unwrap());
            let _attributes = u64::from_le_bytes(entry[48..56].try_into().unwrap());

            // Partition name is UTF-16LE encoded, up to 36 characters
            let name_bytes = &entry[56..128];
            let name: String = name_bytes
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .take_while(|&c| c != 0)
                .filter_map(|c| char::from_u32(c as u32))
                .collect();

            let partition_type = PartitionType::from_gpt_guid(&type_guid);

            partitions.push(PartitionInfo {
                number: i,
                name: if name.is_empty() {
                    format!("Partition {}", i + 1)
                } else {
                    name
                },
                partition_type,
                start_offset: start_lba * 512,
                size: (end_lba - start_lba + 1) * 512,
                uuid: Some(Self::format_guid(&partition_guid)),
                fs_hint: None,
            });
        }

        Ok(Self {
            scheme: PartitionScheme::Gpt,
            partitions,
            disk_size: device.size(),
            block_size: 512,
        })
    }

    /// Try to parse an MBR partition table
    async fn try_parse_mbr(device: &dyn BlockDevice) -> DiskResult<Self> {
        let mut mbr = [0u8; 512];
        device.read_at(&mut mbr, 0).await?;

        // Check MBR signature: 0x55, 0xAA at offset 510
        if mbr[510] != 0x55 || mbr[511] != 0xAA {
            return Err(DiskError::PartitionError {
                reason: "No MBR signature found".to_string(),
            });
        }

        // Check if this is a protective MBR for GPT
        // Protective MBR has type 0xEE covering the whole disk
        if mbr[450] == 0xEE {
            return Err(DiskError::PartitionError {
                reason: "Protective MBR detected (use GPT)".to_string(),
            });
        }

        let mut partitions = Vec::new();

        // Parse 4 primary partition entries at offset 446
        for i in 0..4 {
            let entry_offset = 446 + i * 16;
            let entry = &mbr[entry_offset..entry_offset + 16];

            let type_byte = entry[4];
            if type_byte == 0 {
                continue; // Unused entry
            }

            let start_lba = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as u64;
            let num_sectors = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as u64;

            let partition_type = PartitionType::from_mbr_type(type_byte);

            partitions.push(PartitionInfo {
                number: i as u32,
                name: format!("Partition {}", i + 1),
                partition_type,
                start_offset: start_lba * 512,
                size: num_sectors * 512,
                uuid: None,
                fs_hint: None,
            });

            // Check for extended partition (type 0x05, 0x0F, or 0x85)
            if type_byte == 0x05 || type_byte == 0x0F || type_byte == 0x85 {
                // Parse extended partitions recursively
                let extended_partitions = Self::parse_extended_partitions(
                    device,
                    start_lba,
                    start_lba, // EBR base for relative addressing
                    partitions.len() as u32,
                )
                .await?;
                partitions.extend(extended_partitions);
            }
        }

        Ok(Self {
            scheme: PartitionScheme::Mbr,
            partitions,
            disk_size: device.size(),
            block_size: 512,
        })
    }

    /// Parse extended partitions (logical drives in extended partition)
    ///
    /// Extended partitions use a linked list of Extended Boot Records (EBR).
    /// Each EBR contains:
    /// - Entry 0: Logical partition relative to this EBR
    /// - Entry 1: Next EBR relative to extended partition start
    /// - Entries 2-3: Unused
    async fn parse_extended_partitions(
        device: &dyn BlockDevice,
        ebr_lba: u64,
        extended_start_lba: u64,
        start_number: u32,
    ) -> DiskResult<Vec<PartitionInfo>> {
        let mut partitions = Vec::new();
        let mut current_ebr_lba = ebr_lba;
        let mut partition_num = start_number;

        // Safety limit to prevent infinite loops from corrupt partition tables
        const MAX_LOGICAL_PARTITIONS: usize = 128;

        while partitions.len() < MAX_LOGICAL_PARTITIONS {
            // Read EBR
            let mut ebr = [0u8; 512];
            device.read_at(&mut ebr, current_ebr_lba * 512).await?;

            // Verify EBR signature
            if ebr[510] != 0x55 || ebr[511] != 0xAA {
                break; // End of chain or corrupt
            }

            // Parse entry 0: logical partition (relative to this EBR)
            let entry0 = &ebr[446..462];
            let type0 = entry0[4];

            if type0 != 0 {
                let start_lba0 = u32::from_le_bytes(entry0[8..12].try_into().unwrap()) as u64;
                let num_sectors0 = u32::from_le_bytes(entry0[12..16].try_into().unwrap()) as u64;

                // Logical partition offset is relative to this EBR
                let absolute_start = current_ebr_lba + start_lba0;

                partitions.push(PartitionInfo {
                    number: partition_num,
                    name: format!("Logical Partition {}", partition_num + 1),
                    partition_type: PartitionType::from_mbr_type(type0),
                    start_offset: absolute_start * 512,
                    size: num_sectors0 * 512,
                    uuid: None,
                    fs_hint: None,
                });
                partition_num += 1;
            }

            // Parse entry 1: next EBR (relative to extended partition start)
            let entry1 = &ebr[462..478];
            let type1 = entry1[4];

            if type1 == 0 {
                break; // No more EBRs
            }

            if type1 != 0x05 && type1 != 0x0F && type1 != 0x85 {
                break; // Not an extended partition entry
            }

            let start_lba1 = u32::from_le_bytes(entry1[8..12].try_into().unwrap()) as u64;

            // Next EBR offset is relative to the original extended partition
            current_ebr_lba = extended_start_lba + start_lba1;
        }

        Ok(partitions)
    }

    /// Format a GUID as a string
    fn format_guid(guid: &[u8; 16]) -> String {
        // GUIDs are stored in mixed-endian format
        format!(
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            u32::from_le_bytes([guid[0], guid[1], guid[2], guid[3]]),
            u16::from_le_bytes([guid[4], guid[5]]),
            u16::from_le_bytes([guid[6], guid[7]]),
            guid[8],
            guid[9],
            guid[10],
            guid[11],
            guid[12],
            guid[13],
            guid[14],
            guid[15]
        )
    }

    /// Get all partitions
    pub fn partitions(&self) -> &[PartitionInfo] {
        &self.partitions
    }

    /// Get partitions that should be encoded
    ///
    /// # Why Filter?
    ///
    /// Not all partitions contain useful data for encoding:
    /// - Swap partitions are transient
    /// - Empty partitions waste encoding time
    pub fn encodable_partitions(&self) -> impl Iterator<Item = &PartitionInfo> {
        self.partitions
            .iter()
            .filter(|p| p.partition_type.should_encode())
    }

    /// Get a specific partition by number
    pub fn get_partition(&self, number: u32) -> Option<&PartitionInfo> {
        self.partitions.iter().find(|p| p.number == number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mbr_partition_types() {
        assert_eq!(
            PartitionType::from_mbr_type(0x83),
            PartitionType::LinuxFilesystem
        );
        assert_eq!(PartitionType::from_mbr_type(0x82), PartitionType::LinuxSwap);
        assert_eq!(PartitionType::from_mbr_type(0xEF), PartitionType::EfiSystem);
    }

    #[test]
    fn test_should_encode() {
        assert!(PartitionType::LinuxFilesystem.should_encode());
        assert!(PartitionType::EfiSystem.should_encode());
        assert!(!PartitionType::LinuxSwap.should_encode());
    }
}
