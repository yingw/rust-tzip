//! TorrentZip validation implementation.

use crate::error::{Error, Result, ValidationError};
use crate::spec::{
    CENTRAL_DIR_HEADER_SIG, COMPRESSION_METHOD_DEFLATE, DOS_DATE, DOS_TIME,
    END_OF_CENTRAL_DIR_SIG, GENERAL_PURPOSE_FLAG, UTF8_FLAG,
};
use crc32fast::Hasher;

/// Result of TorrentZip validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the file is a valid TorrentZip
    pub is_valid: bool,
    /// List of validation errors (empty if is_valid)
    pub errors: Vec<ValidationError>,
    /// The TorrentZip CRC32 from the comment (if present)
    pub torrentzip_crc32: Option<u32>,
    /// Computed CRC32 of central directory
    pub computed_crc32: u32,
    /// Number of files in archive
    pub file_count: usize,
    /// File names in archive (lowercase sorted order)
    pub files: Vec<String>,
}

/// TorrentZip validator
pub struct TorrentZipValidator;

impl TorrentZipValidator {
    /// Validate a TorrentZip file from bytes.
    ///
    /// Analyzes the ZIP structure and verifies TorrentZip compliance:
    /// - Fixed timestamp (Dec 24, 1996 23:32:00)
    /// - DEFLATE compression with max level (bit 1 set)
    /// - Files sorted by lowercase name
    /// - Archive comment in format `TORRENTZIPPED-XXXXXXXX`
    /// - Comment CRC32 matches central directory CRC32
    pub fn validate(data: &[u8]) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut files = Vec::new();

        if data.len() < 22 {
            return Ok(ValidationResult {
                is_valid: false,
                errors: vec![ValidationError::InvalidComment],
                torrentzip_crc32: None,
                computed_crc32: 0,
                file_count: 0,
                files: vec![],
            });
        }

        // Find EOCD (search backwards from end)
        let eocd_offset = find_eocd(&data)?;
        let (cd_offset, cd_size, comment) = parse_eocd(&data, eocd_offset)?;

        // Parse and validate archive comment
        let torrentzip_crc32 = parse_torrentzip_comment(&comment);

        if torrentzip_crc32.is_none() {
            errors.push(ValidationError::InvalidComment);
        }

        // Compute CRC32 of central directory
        let computed_crc32 = compute_cd_crc32(&data, cd_offset, cd_size);

        // Validate comment CRC32 matches
        if let Some(expected_crc) = torrentzip_crc32 {
            if expected_crc != computed_crc32 {
                errors.push(ValidationError::CommentCrcMismatch {
                    expected: expected_crc,
                    actual: computed_crc32,
                });
            }
        }

        // Parse central directory entries
        let mut prev_name_lower: Option<String> = None;
        let mut pos = cd_offset as usize;
        let cd_end = (cd_offset + cd_size) as usize;

        while pos < cd_end {
            let (name, entry_size, validation_errs) =
                parse_central_dir_entry(&data, pos)?;
            files.push(name.clone());
            errors.extend(validation_errs);

            // Check sorting by lowercase name
            let name_lower = name.to_lowercase();
            if let Some(prev) = &prev_name_lower {
                if &name_lower < prev {
                    errors.push(ValidationError::FilesNotSorted(
                        prev.clone(),
                        name_lower.clone(),
                    ));
                }
            }
            prev_name_lower = Some(name_lower);

            pos += entry_size;
        }

        let is_valid = errors.is_empty();

        Ok(ValidationResult {
            is_valid,
            errors,
            torrentzip_crc32,
            computed_crc32,
            file_count: files.len(),
            files,
        })
    }
}

/// Find EOCD by searching backwards from end of file.
fn find_eocd(data: &[u8]) -> Result<usize> {
    // EOCD signature
    let sig: [u8; 4] = END_OF_CENTRAL_DIR_SIG.to_le_bytes();

    // Search backwards, accounting for possible comment (max 65535 bytes + 22 byte EOCD)
    let max_comment = 65535usize;
    let search_start = data.len().saturating_sub(22 + max_comment);

    for i in search_start..data.len().saturating_sub(21) {
        if data[i..i + 4] == sig {
            return Ok(i);
        }
    }

    Err(Error::InvalidZip("EOCD not found".to_string()))
}

/// Parse EOCD and return (cd_offset, cd_size, comment)
fn parse_eocd(data: &[u8], offset: usize) -> Result<(u32, u32, Vec<u8>)> {
    if offset + 22 > data.len() {
        return Err(Error::InvalidZip("EOCD truncated".to_string()));
    }

    let cd_size = u32::from_le_bytes([data[offset + 12], data[offset + 13], data[offset + 14], data[offset + 15]]);
    let cd_offset = u32::from_le_bytes([data[offset + 16], data[offset + 17], data[offset + 18], data[offset + 19]]);
    let comment_len = u16::from_le_bytes([data[offset + 20], data[offset + 21]]) as usize;

    let comment = if offset + 22 + comment_len <= data.len() {
        data[offset + 22..offset + 22 + comment_len].to_vec()
    } else {
        vec![]
    };

    Ok((cd_offset, cd_size, comment))
}

/// Parse TorrentZip comment and extract CRC32.
/// Returns None if comment is not in correct format.
fn parse_torrentzip_comment(comment: &[u8]) -> Option<u32> {
    let prefix = b"TORRENTZIPPED-";
    if comment.len() != 22 {
        return None;
    }
    if !comment.starts_with(prefix) {
        return None;
    }

    // Parse hex CRC32
    let hex_str = std::str::from_utf8(&comment[14..22]).ok()?;
    u32::from_str_radix(hex_str, 16).ok()
}

/// Compute CRC32 of central directory bytes.
fn compute_cd_crc32(data: &[u8], offset: u32, size: u32) -> u32 {
    let start = offset as usize;
    let end = start + size as usize;

    if end > data.len() {
        return 0;
    }

    let mut hasher = Hasher::new();
    hasher.update(&data[start..end]);
    hasher.finalize()
}

/// Parse a central directory entry and validate TorrentZip compliance.
/// Returns (filename, entry_size, validation_errors)
fn parse_central_dir_entry(
    data: &[u8],
    offset: usize,
) -> Result<(String, usize, Vec<ValidationError>)> {
    let mut errors = Vec::new();

    if offset + 46 > data.len() {
        return Err(Error::InvalidZip("Central directory entry truncated".to_string()));
    }

    // Verify signature
    let sig = u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]);
    if sig != CENTRAL_DIR_HEADER_SIG {
        return Err(Error::InvalidZip("Invalid central directory signature".to_string()));
    }

    // Extract fields
    let compression = u16::from_le_bytes([data[offset + 10], data[offset + 11]]);
    let mod_time = u16::from_le_bytes([data[offset + 12], data[offset + 13]]);
    let mod_date = u16::from_le_bytes([data[offset + 14], data[offset + 15]]);
    let filename_len = u16::from_le_bytes([data[offset + 28], data[offset + 29]]) as usize;
    let extra_len = u16::from_le_bytes([data[offset + 30], data[offset + 31]]) as usize;
    let comment_len = u16::from_le_bytes([data[offset + 32], data[offset + 33]]) as usize;

    // General purpose flag (for compression level validation)
    let general_flag = u16::from_le_bytes([data[offset + 8], data[offset + 9]]);

    // Validate TorrentZip requirements
    if compression != COMPRESSION_METHOD_DEFLATE {
        errors.push(ValidationError::WrongCompressionMethod(compression));
    }

    // Check general purpose bit flag
    // Bit 1 (0x0002) must be set for max compression
    // Bit 11 (0x0800) is the UTF-8 flag (EFS) - set conditionally for non-ASCII filenames
    // Accept both 0x0002 (ASCII filenames) and 0x0802 (non-ASCII filenames)
    if general_flag != GENERAL_PURPOSE_FLAG && general_flag != GENERAL_PURPOSE_FLAG | UTF8_FLAG {
        errors.push(ValidationError::WrongGeneralFlag(general_flag));
    }

    // Check timestamp
    if mod_time != DOS_TIME || mod_date != DOS_DATE {
        errors.push(ValidationError::WrongTimestamp);
    }

    // Check for extra data (not allowed in TorrentZip)
    if extra_len > 0 {
        errors.push(ValidationError::ExtraDataPresent);
    }

    // Check for file comments (not allowed in TorrentZip)
    if comment_len > 0 {
        errors.push(ValidationError::FileCommentsPresent);
    }

    // Extract filename
    let name_start = offset + 46;
    let name_end = name_start + filename_len;
    if name_end > data.len() {
        return Err(Error::InvalidZip("Filename truncated".to_string()));
    }

    let filename = String::from_utf8_lossy(&data[name_start..name_end]).to_string();

    let entry_size = 46 + filename_len + extra_len + comment_len;

    Ok((filename, entry_size, errors))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_torrentzip_comment() {
        let comment = b"TORRENTZIPPED-F175FDED";
        assert_eq!(parse_torrentzip_comment(comment), Some(0xF175FDED));

        let bad_comment = b"NOT A TORRENTZIP";
        assert_eq!(parse_torrentzip_comment(bad_comment), None);

        let wrong_len = b"TORRENTZIPPED-123";
        assert_eq!(parse_torrentzip_comment(wrong_len), None);
    }
}
