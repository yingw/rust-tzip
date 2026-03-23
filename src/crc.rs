//! Central directory CRC32 computation for TorrentZip format.

use crc32fast::Hasher;

/// Compute CRC32 of the ZIP file's central directory headers.
///
/// The central directory starts after all local file headers + compressed data,
/// and before the end of central directory record (EOCD).
///
/// # Arguments
///
/// * `zip_data` - Complete ZIP file bytes
///
/// # Returns
///
/// CRC32 value of the central directory, or 0 if the ZIP structure is invalid.
pub fn compute_central_directory_crc32(zip_data: &[u8]) -> u32 {
    // EOCD signature: PK\x05\x06
    const EOCD_SIG: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];

    // Minimum EOCD size is 22 bytes
    if zip_data.len() < 22 {
        return 0;
    }

    // EOCD can have a variable-length comment (max 65535 bytes)
    // Search backwards from end for EOCD signature
    // Comment length is at offset 20-21
    let max_comment_len = 65535u16;
    let search_end = zip_data.len();
    let search_start = search_end.saturating_sub(22 + max_comment_len as usize + 1);

    let mut eocd_offset = None;
    for i in search_start..search_end {
        if zip_data[i..i + 4] == EOCD_SIG {
            eocd_offset = Some(i);
            break;
        }
    }

    let eocd_offset = match eocd_offset {
        Some(offset) => offset,
        None => return 0,
    };

    // Parse EOCD to find central directory offset and size
    // EOCD structure (after 4-byte signature):
    // Bytes 0-1: Disk number (always 0 for non-spanning)
    // Bytes 2-3: Disk with central directory start (always 0)
    // Bytes 4-5: Entries on this disk
    // Bytes 6-7: Total entries
    // Bytes 8-11: Central directory size (4 bytes, LE)
    // Bytes 12-15: Central directory offset (4 bytes, LE)

    let cd_size = u32::from_le_bytes([
        zip_data[eocd_offset + 12],
        zip_data[eocd_offset + 13],
        zip_data[eocd_offset + 14],
        zip_data[eocd_offset + 15],
    ]);

    let cd_offset = u32::from_le_bytes([
        zip_data[eocd_offset + 16],
        zip_data[eocd_offset + 17],
        zip_data[eocd_offset + 18],
        zip_data[eocd_offset + 19],
    ]);

    // Validate bounds
    let cd_offset_usize = cd_offset as usize;
    let cd_end = cd_offset_usize + cd_size as usize;
    if cd_end > zip_data.len() {
        return 0;
    }

    // Compute CRC32 of central directory bytes
    let mut hasher = Hasher::new();
    hasher.update(&zip_data[cd_offset_usize..cd_end]);
    hasher.finalize()
}

