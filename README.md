# tzip

A Rust library for creating and validating TorrentZip-format ZIP files.

## What is TorrentZip?

TorrentZip is a ZIP file format specification that ensures **byte-identical output** across different systems and tools. This is critical for:

- **ROM preservation** - Verify file integrity with hash comparison
- **Distributed systems** - BitTorrent/P2P sharing benefits from identical files
- **Reproducible builds** - Same input always produces the same output

## Features

- ✅ **Byte-identical output** - Matches original trrntzip tools exactly
- ✅ **Full validation** - Verify existing TorrentZip files
- ✅ **Zero unsafe code** - 100% safe Rust
- ✅ **No dependencies on external tools** - Pure Rust + zlib

## Usage

### Creating a TorrentZip

```rust
use tzip::TorrentZipWriter;

let mut buffer = Vec::new();
{
    let mut tz = TorrentZipWriter::new(&mut buffer);
    tz.add_file("game.bin", &rom_data)?;
    tz.add_file("bios.bin", &bios_data)?;
    tz.finish()?;
}

// buffer now contains a valid TorrentZip file
std::fs::write("output.zip", &buffer)?;
```

### Validating a TorrentZip

```rust
use tzip::TorrentZipValidator;

let data = std::fs::read("archive.zip")?;
let result = TorrentZipValidator::validate(&data)?;

if result.is_valid {
    println!("Valid TorrentZip! CRC32: {:08X}", result.computed_crc32);
} else {
    for error in result.errors {
        eprintln!("Validation error: {:?}", error);
    }
}
```

### Convenience Function

```rust
use tzip::create_torrentzip;

let files = vec![
    ("rom1.bin", &rom1_data[..]),
    ("rom2.bin", &rom2_data[..]),
];

let zip_data = create_torrentzip(files)?;
```

## TorrentZip Specification

This crate implements the full TorrentZip specification:

| Property | Value |
|----------|-------|
| Compression | DEFLATE (level 9, max compression) |
| Timestamp | Fixed: December 24, 1996 23:32:00 |
| File order | Sorted by lowercase filename |
| Archive comment | `TORRENTZIPPED-{CRC32}` (uppercase hex) |
| CRC32 source | CRC32 of central directory headers |

### Why These Constraints?

- **Fixed timestamp** - Eliminates time-based differences between builds
- **Max compression** - Consistent DEFLATE output via zlib
- **Sorted files** - Deterministic file ordering
- **Archive comment** - Self-verifying checksum of structure

## Testing

The test suite includes 21 tests covering:

- Byte-identical output comparison with original trrntzip
- Determinism (same input → same output, run 5 times)
- Validation of valid TorrentZip files
- Detection of invalid/corrupted files
- Case-insensitive file sorting

```bash
cargo test --release
```

## Compatibility

This crate produces **byte-identical output** to the original C `trrntzip` tool by using:

- `flate2` with `zlib` backend (same DEFLATE implementation)
- Matching `VERSION_MADE_BY = 0` in central directory headers

Files created by this crate are fully compatible with:
- `trrntzip` (original C tool)
- `trrntzip` .NET port
- Any standard ZIP utility

## License

MIT OR Apache-2.0

## Credits

Based on the [TorrentZip specification](https://github.com/tikki/trrntzip) and PKWARE APPNOTE 6.3.0.
