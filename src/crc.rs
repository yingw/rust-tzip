//! Central directory CRC32 computation for TorrentZip format.

use crc32fast::Hasher;

/// Compute CRC32 of the central directory headers only.
///
/// This takes the raw central directory bytes (not including EOCD).
///
/// # Arguments
///
/// * `cd_data` - Central directory bytes (from first header to last, no EOCD)
///
/// # Returns
///
/// CRC32 value of the central directory bytes.
pub fn compute_central_directory_crc32(cd_data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(cd_data);
    hasher.finalize()
}

