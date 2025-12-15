//! Compression utilities for cache entries.

use crate::types::CompressionType;
use oxide_core::Result;
use std::io::{Read, Write};

/// Compress data using the specified algorithm.
pub fn compress(data: &[u8], algorithm: CompressionType) -> Result<Vec<u8>> {
    match algorithm {
        CompressionType::None => Ok(data.to_vec()),
        CompressionType::Zstd => compress_zstd(data),
        CompressionType::Gzip => compress_gzip(data),
        CompressionType::Lz4 => compress_lz4(data),
    }
}

/// Decompress data using the specified algorithm.
pub fn decompress(data: &[u8], algorithm: CompressionType) -> Result<Vec<u8>> {
    match algorithm {
        CompressionType::None => Ok(data.to_vec()),
        CompressionType::Zstd => decompress_zstd(data),
        CompressionType::Gzip => decompress_gzip(data),
        CompressionType::Lz4 => decompress_lz4(data),
    }
}

fn compress_zstd(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = zstd::Encoder::new(Vec::new(), 3)
        .map_err(|e| oxide_core::Error::Internal(format!("Zstd compression failed: {}", e)))?;
    encoder
        .write_all(data)
        .map_err(|e| oxide_core::Error::Internal(format!("Zstd write failed: {}", e)))?;
    encoder
        .finish()
        .map_err(|e| oxide_core::Error::Internal(format!("Zstd finish failed: {}", e)))
}

fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = zstd::Decoder::new(data)
        .map_err(|e| oxide_core::Error::Internal(format!("Zstd decompression failed: {}", e)))?;
    let mut output = Vec::new();
    decoder
        .read_to_end(&mut output)
        .map_err(|e| oxide_core::Error::Internal(format!("Zstd read failed: {}", e)))?;
    Ok(output)
}

fn compress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(data)
        .map_err(|e| oxide_core::Error::Internal(format!("Gzip write failed: {}", e)))?;
    encoder
        .finish()
        .map_err(|e| oxide_core::Error::Internal(format!("Gzip finish failed: {}", e)))
}

fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut output = Vec::new();
    decoder
        .read_to_end(&mut output)
        .map_err(|e| oxide_core::Error::Internal(format!("Gzip read failed: {}", e)))?;
    Ok(output)
}

fn compress_lz4(data: &[u8]) -> Result<Vec<u8>> {
    lz4_flex::compress_prepend_size(data);
    Ok(lz4_flex::compress_prepend_size(data))
}

fn decompress_lz4(data: &[u8]) -> Result<Vec<u8>> {
    lz4_flex::decompress_size_prepended(data)
        .map_err(|e| oxide_core::Error::Internal(format!("LZ4 decompression failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zstd_roundtrip() {
        let data = b"Hello, World! This is a test of compression.";
        let compressed = compress(data, CompressionType::Zstd).unwrap();
        let decompressed = decompress(&compressed, CompressionType::Zstd).unwrap();
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_gzip_roundtrip() {
        let data = b"Hello, World! This is a test of compression.";
        let compressed = compress(data, CompressionType::Gzip).unwrap();
        let decompressed = decompress(&compressed, CompressionType::Gzip).unwrap();
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_lz4_roundtrip() {
        let data = b"Hello, World! This is a test of compression.";
        let compressed = compress(data, CompressionType::Lz4).unwrap();
        let decompressed = decompress(&compressed, CompressionType::Lz4).unwrap();
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }
}
