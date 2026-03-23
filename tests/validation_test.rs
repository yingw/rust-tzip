//! Comprehensive TorrentZip validation tests

use std::fs;
use std::path::PathBuf;
use tzip::{TorrentZipValidator, TorrentZipWriter};

fn test_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

// ============================================================================
// BYTE-IDENTICAL TESTS
// ============================================================================

#[test]
fn test_byte_identical_single_file() {
    let original = fs::read(test_dir().join("22vp931_orig.zip")).expect("Failed to read original");
    let file_data = fs::read(test_dir().join("at-6-1_a.bin")).expect("Failed to read test data");

    let mut buffer = Vec::new();
    {
        let mut tz = TorrentZipWriter::new(&mut buffer);
        tz.add_file("at-6-1_a.bin", &file_data).expect("Failed to add file");
        tz.finish().expect("Failed to finish");
    }

    assert_eq!(original.len(), buffer.len(), "ZIP sizes should match");
    assert_eq!(original, buffer, "Generated ZIP should match original byte-for-byte");
}

#[test]
fn test_byte_identical_empty_file() {
    // Create a TorrentZip with an empty file
    let mut buffer1 = Vec::new();
    {
        let mut tz = TorrentZipWriter::new(&mut buffer1);
        tz.add_file("empty.txt", &[]).expect("Failed to add empty file");
        tz.finish().expect("Failed to finish");
    }

    // Should produce deterministic output
    let mut buffer2 = Vec::new();
    {
        let mut tz = TorrentZipWriter::new(&mut buffer2);
        tz.add_file("empty.txt", &[]).expect("Failed to add empty file");
        tz.finish().expect("Failed to finish");
    }

    assert_eq!(buffer1, buffer2, "Empty file should produce identical output");
}

#[test]
fn test_byte_identical_multiple_files() {
    // Create a TorrentZip with multiple files
    let file1 = b"Hello, World! This is file 1.";
    let file2 = b"This is file 2 with different content.";
    let file3 = b"And this is file 3.";

    let mut buffer1 = Vec::new();
    {
        let mut tz = TorrentZipWriter::new(&mut buffer1);
        tz.add_file("z_file.txt", file3).expect("Failed to add file");
        tz.add_file("a_file.txt", file1).expect("Failed to add file");
        tz.add_file("m_file.txt", file2).expect("Failed to add file");
        tz.finish().expect("Failed to finish");
    }

    // Generate again - should be identical
    let mut buffer2 = Vec::new();
    {
        let mut tz = TorrentZipWriter::new(&mut buffer2);
        tz.add_file("z_file.txt", file3).expect("Failed to add file");
        tz.add_file("a_file.txt", file1).expect("Failed to add file");
        tz.add_file("m_file.txt", file2).expect("Failed to add file");
        tz.finish().expect("Failed to finish");
    }

    assert_eq!(buffer1, buffer2, "Multiple files should produce identical output");

    // Verify files are sorted by lowercase name
    let result = TorrentZipValidator::validate(&buffer1[..]).expect("Validation failed");
    assert!(result.is_valid, "Should be valid TorrentZip");
    assert_eq!(result.files, vec!["a_file.txt", "m_file.txt", "z_file.txt"]);
}

// ============================================================================
// VALIDATION TESTS - VALID TORRENTZIP
// ============================================================================

#[test]
fn test_validate_known_good_torrentzip() {
    let original = fs::read(test_dir().join("22vp931_orig.zip")).expect("Failed to read original");

    let result = TorrentZipValidator::validate(&original[..]).expect("Validation failed");

    eprintln!("Validation result: {:?}", result);

    assert!(result.is_valid, "Known-good TorrentZip should validate");
    assert!(result.torrentzip_crc32.is_some());
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files, vec!["at-6-1_a.bin"]);
}

#[test]
fn test_validate_generated_torrentzip() {
    let file_data = fs::read(test_dir().join("at-6-1_a.bin")).expect("Failed to read test data");

    let mut buffer = Vec::new();
    {
        let mut tz = TorrentZipWriter::new(&mut buffer);
        tz.add_file("at-6-1_a.bin", &file_data).expect("Failed to add file");
        tz.finish().expect("Failed to finish");
    }

    let result = TorrentZipValidator::validate(&buffer[..]).expect("Validation failed");

    assert!(result.is_valid, "Generated TorrentZip should validate");
    assert!(result.torrentzip_crc32.is_some());
    assert_eq!(result.files.len(), 1);
}

#[test]
fn test_validate_crc32_matches() {
    let file_data = fs::read(test_dir().join("at-6-1_a.bin")).expect("Failed to read test data");

    let mut buffer = Vec::new();
    let mut tz = TorrentZipWriter::new(&mut buffer);
    tz.add_file("at-6-1_a.bin", &file_data).expect("Failed to add file");
    tz.finish().expect("Failed to finish");
    let tz_crc = tz.torrentzip_crc32().expect("Should have CRC32");

    let result = TorrentZipValidator::validate(&buffer[..]).expect("Validation failed");

    assert!(result.is_valid);
    assert_eq!(result.torrentzip_crc32, Some(tz_crc));
    assert_eq!(result.computed_crc32, tz_crc);
}

// ============================================================================
// VALIDATION TESTS - INVALID TORRENTZIP
// ============================================================================

#[test]
fn test_validate_non_torrentzip_no_comment() {
    // Create a minimal ZIP without TorrentZip comment using raw bytes
    // This is a stored (not compressed) ZIP without the proper comment
    let mut buffer: Vec<u8> = Vec::new();

    // Local file header
    buffer.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]); // signature
    buffer.extend_from_slice(&[0x0a, 0x00]); // version needed
    buffer.extend_from_slice(&[0x00, 0x00]); // general flag
    buffer.extend_from_slice(&[0x00, 0x00]); // stored
    buffer.extend_from_slice(&[0x00, 0x00]); // time
    buffer.extend_from_slice(&[0x00, 0x00]); // date
    buffer.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // crc32
    buffer.extend_from_slice(&[0x04, 0x00, 0x00, 0x00]); // compressed size
    buffer.extend_from_slice(&[0x04, 0x00, 0x00, 0x00]); // uncompressed size
    buffer.extend_from_slice(&[0x04, 0x00]); // filename len
    buffer.extend_from_slice(&[0x00, 0x00]); // extra len
    buffer.extend_from_slice(b"test"); // filename
    buffer.extend_from_slice(b"data"); // file data

    // Central directory
    let cd_offset = buffer.len();
    buffer.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]); // signature
    buffer.extend_from_slice(&[0x00, 0x00]); // version made by
    buffer.extend_from_slice(&[0x0a, 0x00]); // version needed
    buffer.extend_from_slice(&[0x00, 0x00]); // general flag
    buffer.extend_from_slice(&[0x00, 0x00]); // stored
    buffer.extend_from_slice(&[0x00, 0x00]); // time
    buffer.extend_from_slice(&[0x00, 0x00]); // date
    buffer.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // crc32
    buffer.extend_from_slice(&[0x04, 0x00, 0x00, 0x00]); // compressed size
    buffer.extend_from_slice(&[0x04, 0x00, 0x00, 0x00]); // uncompressed size
    buffer.extend_from_slice(&[0x04, 0x00]); // filename len
    buffer.extend_from_slice(&[0x00, 0x00]); // extra len
    buffer.extend_from_slice(&[0x00, 0x00]); // comment len
    buffer.extend_from_slice(&[0x00, 0x00]); // disk start
    buffer.extend_from_slice(&[0x00, 0x00]); // internal attr
    buffer.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // external attr
    buffer.extend_from_slice(&(0u32).to_le_bytes()); // local offset
    buffer.extend_from_slice(b"test"); // filename

    let cd_size = buffer.len() - cd_offset;

    // EOCD
    buffer.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]); // signature
    buffer.extend_from_slice(&[0x00, 0x00]); // disk num
    buffer.extend_from_slice(&[0x00, 0x00]); // cd disk
    buffer.extend_from_slice(&[0x01, 0x00]); // entries on disk
    buffer.extend_from_slice(&[0x01, 0x00]); // total entries
    buffer.extend_from_slice(&(cd_size as u32).to_le_bytes()); // cd size
    buffer.extend_from_slice(&(cd_offset as u32).to_le_bytes()); // cd offset
    buffer.extend_from_slice(&[0x00, 0x00]); // comment len

    let result = TorrentZipValidator::validate(&buffer[..]).expect("Validation failed");

    assert!(!result.is_valid, "Non-TorrentZip should fail validation");
    assert!(result.errors.iter().any(|e| matches!(e, tzip::ValidationError::InvalidComment)));
}

#[test]
fn test_validate_corrupted_crc() {
    let file_data = fs::read(test_dir().join("at-6-1_a.bin")).expect("Failed to read test data");

    let mut buffer = Vec::new();
    {
        let mut tz = TorrentZipWriter::new(&mut buffer);
        tz.add_file("at-6-1_a.bin", &file_data).expect("Failed to add file");
        tz.finish().expect("Failed to finish");
    }

    // Corrupt the CRC32 in the comment by changing one hex digit to another valid hex
    // Comment is "TORRENTZIPPED-XXXXXXXX" (22 bytes at end of file)
    // Change first hex digit of CRC32
    let comment_start = buffer.len() - 22;
    let crc_start = comment_start + 14; // Position of first CRC hex digit
    
    // Swap 'F' <-> '0' (both valid hex)
    if buffer[crc_start] == b'F' {
        buffer[crc_start] = b'0';
    } else {
        buffer[crc_start] = b'F';
    }

    let result = TorrentZipValidator::validate(&buffer[..]).expect("Validation failed");

    assert!(!result.is_valid, "Corrupted CRC should fail validation");
    assert!(
        result.errors.iter().any(|e| matches!(e, tzip::ValidationError::CommentCrcMismatch { .. })),
        "Should have CommentCrcMismatch error, got: {:?}",
        result.errors
    );
}

#[test]
fn test_validate_truncated_file() {
    let data = [0x50, 0x4b]; // Just the start of a ZIP signature

    let result = TorrentZipValidator::validate(&data[..]).expect("Validation should not crash");

    assert!(!result.is_valid);
}

#[test]
fn test_validate_empty_file() {
    let data: [u8; 0] = [];

    let result = TorrentZipValidator::validate(&data[..]).expect("Validation should not crash");

    assert!(!result.is_valid);
}

// ============================================================================
// DETERMINISM TESTS
// ============================================================================

#[test]
fn test_determinism_same_content() {
    let file_data = fs::read(test_dir().join("at-6-1_a.bin")).expect("Failed to read test data");

    // Generate 5 times
    let outputs: Vec<Vec<u8>> = (0..5)
        .map(|_| {
            let mut buffer = Vec::new();
            let mut tz = TorrentZipWriter::new(&mut buffer);
            tz.add_file("at-6-1_a.bin", &file_data).expect("Failed to add file");
            tz.finish().expect("Failed to finish");
            buffer
        })
        .collect();

    // All should be identical
    for (i, output) in outputs.iter().enumerate().skip(1) {
        assert_eq!(
            outputs[0], *output,
            "Output {} should be identical to output 0",
            i
        );
    }
}

#[test]
fn test_determinism_case_sorting() {
    // Files with different cases should be sorted case-insensitively
    let mut buffer1 = Vec::new();
    {
        let mut tz = TorrentZipWriter::new(&mut buffer1);
        tz.add_file("Z_FILE.BIN", b"data1").expect("Failed to add file");
        tz.add_file("a_file.bin", b"data2").expect("Failed to add file");
        tz.add_file("M_File.Bin", b"data3").expect("Failed to add file");
        tz.finish().expect("Failed to finish");
    }

    let mut buffer2 = Vec::new();
    {
        let mut tz = TorrentZipWriter::new(&mut buffer2);
        // Add in different order
        tz.add_file("M_File.Bin", b"data3").expect("Failed to add file");
        tz.add_file("Z_FILE.BIN", b"data1").expect("Failed to add file");
        tz.add_file("a_file.bin", b"data2").expect("Failed to add file");
        tz.finish().expect("Failed to finish");
    }

    assert_eq!(buffer1, buffer2, "Files should be sorted case-insensitively");

    // Validate the result
    let result = TorrentZipValidator::validate(&buffer1[..]).expect("Validation failed");
    assert!(result.is_valid);
    // Sorted by lowercase: a_file.bin, M_File.Bin, Z_FILE.BIN
    assert_eq!(result.files, vec!["a_file.bin", "M_File.Bin", "Z_FILE.BIN"]);
}
