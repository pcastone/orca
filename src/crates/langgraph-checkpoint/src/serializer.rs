//! Serialization protocol for checkpoints

use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Protocol for serializing and deserializing checkpoint data
///
/// Implementations can provide custom serialization strategies
/// (JSON, MessagePack, bincode, etc.)
pub trait SerializerProtocol: Send + Sync {
    /// Serialize a value to bytes
    fn dumps<T: Serialize>(&self, value: &T) -> Result<Vec<u8>>;

    /// Deserialize a value from bytes
    fn loads<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Result<T>;

    /// Serialize to JSON value (for compatibility)
    fn dumps_json<T: Serialize>(&self, value: &T) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(value)?)
    }

    /// Deserialize from JSON value (for compatibility)
    fn loads_json<T: for<'de> Deserialize<'de>>(&self, value: &serde_json::Value) -> Result<T> {
        Ok(serde_json::from_value(value.clone())?)
    }
}

/// JSON-based serializer (default)
#[derive(Debug, Clone, Default)]
pub struct JsonSerializer;

impl JsonSerializer {
    pub fn new() -> Self {
        Self
    }
}

impl SerializerProtocol for JsonSerializer {
    fn dumps<T: Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(value)?)
    }

    fn loads<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Result<T> {
        Ok(serde_json::from_slice(data)?)
    }
}

/// Binary serializer using bincode
#[derive(Debug, Clone, Default)]
pub struct BincodeSerializer;

impl BincodeSerializer {
    pub fn new() -> Self {
        Self
    }
}

impl SerializerProtocol for BincodeSerializer {
    fn dumps<T: Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        Ok(bincode::serialize(value)?)
    }

    fn loads<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Result<T> {
        Ok(bincode::deserialize(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn test_json_serializer() {
        let serializer = JsonSerializer::new();
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let bytes = serializer.dumps(&data).unwrap();
        let restored: TestData = serializer.loads(&bytes).unwrap();

        assert_eq!(data, restored);
    }

    #[test]
    fn test_bincode_serializer() {
        let serializer = BincodeSerializer::new();
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let bytes = serializer.dumps(&data).unwrap();
        let restored: TestData = serializer.loads(&bytes).unwrap();

        assert_eq!(data, restored);
    }

    #[test]
    fn test_json_value_serialization() {
        let serializer = JsonSerializer::new();
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let json = serializer.dumps_json(&data).unwrap();
        let restored: TestData = serializer.loads_json(&json).unwrap();

        assert_eq!(data, restored);
    }
}
