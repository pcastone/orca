//! WebSocket message compression
//!
//! Provides optional message compression to reduce bandwidth usage.

use flate2::Compression;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use std::io::{Read, Write};

/// Compression level
#[derive(Debug, Clone, Copy)]
pub enum CompressionLevel {
    /// No compression
    None,
    /// Fast compression (level 1)
    Fast,
    /// Default compression (level 6)
    Default,
    /// Maximum compression (level 9)
    Maximum,
}

impl CompressionLevel {
    /// Convert to flate2 compression level
    fn to_flate2(&self) -> Compression {
        match self {
            CompressionLevel::None => Compression::none(),
            CompressionLevel::Fast => Compression::fast(),
            CompressionLevel::Default => Compression::default(),
            CompressionLevel::Maximum => Compression::best(),
        }
    }
}

/// Message compression handler
pub struct MessageCompressor {
    /// Compression level
    level: CompressionLevel,
    /// Minimum message size to compress (bytes)
    min_size: usize,
}

impl MessageCompressor {
    /// Create new compressor
    pub fn new(level: CompressionLevel, min_size: usize) -> Self {
        Self { level, min_size }
    }

    /// Create with default settings (default compression, 1KB minimum)
    pub fn new_default() -> Self {
        Self {
            level: CompressionLevel::Default,
            min_size: 1024,
        }
    }

    /// Create with fast compression
    pub fn new_fast() -> Self {
        Self {
            level: CompressionLevel::Fast,
            min_size: 1024,
        }
    }

    /// Create with maximum compression
    pub fn new_maximum() -> Self {
        Self {
            level: CompressionLevel::Maximum,
            min_size: 1024,
        }
    }

    /// Compress a message
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        // Don't compress if too small or compression disabled
        if data.len() < self.min_size || matches!(self.level, CompressionLevel::None) {
            return Ok(data.to_vec());
        }

        let mut encoder = GzEncoder::new(Vec::new(), self.level.to_flate2());
        encoder
            .write_all(data)
            .map_err(|e| format!("Compression failed: {}", e))?;

        encoder
            .finish()
            .map_err(|e| format!("Compression finish failed: {}", e))
    }

    /// Decompress a message
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();

        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| format!("Decompression failed: {}", e))?;

        Ok(decompressed)
    }

    /// Calculate compression ratio
    pub fn compression_ratio(&self, original: &[u8], compressed: &[u8]) -> f64 {
        if original.is_empty() {
            return 0.0;
        }
        (compressed.len() as f64 / original.len() as f64) * 100.0
    }

    /// Check if data is compressed
    pub fn is_compressed(data: &[u8]) -> bool {
        // GZIP magic number is 0x1f 0x8b
        data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b
    }
}

impl Default for MessageCompressor {
    fn default() -> Self {
        Self::new_default()
    }
}

/// Compression statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompressionStats {
    /// Messages compressed
    pub messages_compressed: u64,
    /// Messages not compressed (too small)
    pub messages_uncompressed: u64,
    /// Total bytes before compression
    pub bytes_original: u64,
    /// Total bytes after compression
    pub bytes_compressed: u64,
}

impl CompressionStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            messages_compressed: 0,
            messages_uncompressed: 0,
            bytes_original: 0,
            bytes_compressed: 0,
        }
    }

    /// Get overall compression ratio
    pub fn overall_ratio(&self) -> f64 {
        if self.bytes_original == 0 {
            return 0.0;
        }
        (self.bytes_compressed as f64 / self.bytes_original as f64) * 100.0
    }

    /// Get bytes saved
    pub fn bytes_saved(&self) -> u64 {
        self.bytes_original.saturating_sub(self.bytes_compressed)
    }
}

impl Default for CompressionStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressor_creation() {
        let compressor = MessageCompressor::new_default();
        assert!(matches!(compressor.level, CompressionLevel::Default));
    }

    #[test]
    fn test_compress_small_message() {
        let compressor = MessageCompressor::new(CompressionLevel::Default, 1024);
        let data = b"hello world";

        // Small messages should return as-is
        let compressed = compressor.compress(data).unwrap();
        assert_eq!(compressed, data);
    }

    #[test]
    fn test_compress_large_message() {
        let compressor = MessageCompressor::new(CompressionLevel::Default, 10);
        let data = vec![b'a'; 1000];

        let compressed = compressor.compress(&data).unwrap();
        // Compressed should be smaller for repetitive data
        // Note: gzip compression works best with repetitive data
        assert!(compressed.len() <= data.len());
    }

    #[test]
    fn test_decompress() {
        let compressor = MessageCompressor::new_default();
        let original = b"hello world, this is a test message";
        let mut original_vec = Vec::new();
        for i in 0..100 {
            if i > 0 {
                original_vec.push(b' ');
            }
            original_vec.extend_from_slice(original);
        }

        let compressed = compressor.compress(&original_vec).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(decompressed, original_vec);
    }

    #[test]
    fn test_compression_ratio() {
        let compressor = MessageCompressor::new_default();
        let original = vec![b'a'; 1000];
        let compressed = vec![b'b'; 100];

        let ratio = compressor.compression_ratio(&original, &compressed);
        assert!((ratio - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_is_compressed() {
        // GZIP magic number
        let gzip_data = vec![0x1f, 0x8b];
        assert!(MessageCompressor::is_compressed(&gzip_data));

        let plain_data = vec![0x00, 0x00];
        assert!(!MessageCompressor::is_compressed(&plain_data));
    }

    #[test]
    fn test_compression_disabled() {
        let compressor = MessageCompressor::new(CompressionLevel::None, 0);
        let data = b"this should not be compressed even though it's large";
        let data_large = vec![data.as_ref(); 1000].join(&b" "[..]);

        let compressed = compressor.compress(&data_large).unwrap();
        assert_eq!(compressed, data_large);
    }

    #[test]
    fn test_compression_stats() {
        let stats = CompressionStats {
            messages_compressed: 10,
            messages_uncompressed: 5,
            bytes_original: 1000,
            bytes_compressed: 500,
        };

        assert!(stats.overall_ratio() < 100.0);
        assert_eq!(stats.bytes_saved(), 500);
    }
}
