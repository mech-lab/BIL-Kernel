use bil_core::DigestSet;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Write;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HashError {
    #[error("failed to serialize value into JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("failed to parse canonical JSON input: {0}")]
    Parse(#[source] serde_json::Error),
    #[error("failed to render canonical JSON: {0}")]
    Render(#[source] serde_json::Error),
    #[error("failed to write canonical JSON bytes: {0}")]
    Io(#[source] std::io::Error),
}

pub fn canonical_json_bytes<T>(value: &T) -> Result<Vec<u8>, HashError>
where
    T: Serialize,
{
    let value = serde_json::to_value(value).map_err(HashError::Serialize)?;
    canonical_json_value_bytes(&value)
}

pub fn canonical_json_slice(bytes: &[u8]) -> Result<Vec<u8>, HashError> {
    let value: Value = serde_json::from_slice(bytes).map_err(HashError::Parse)?;
    canonical_json_value_bytes(&value)
}

pub fn digest_bytes(bytes: &[u8]) -> DigestSet {
    let sha256 = {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    };
    let blake3 = blake3::hash(bytes).to_hex().to_string();

    DigestSet { sha256, blake3 }
}

fn canonical_json_value_bytes(value: &Value) -> Result<Vec<u8>, HashError> {
    let mut output = Vec::new();
    write_canonical_json(&mut output, value)?;
    Ok(output)
}

fn write_canonical_json<W>(writer: &mut W, value: &Value) -> Result<(), HashError>
where
    W: Write,
{
    match value {
        Value::Null => writer.write_all(b"null").map_err(HashError::Io)?,
        Value::Bool(true) => writer.write_all(b"true").map_err(HashError::Io)?,
        Value::Bool(false) => writer.write_all(b"false").map_err(HashError::Io)?,
        Value::Number(number) => writer
            .write_all(number.to_string().as_bytes())
            .map_err(HashError::Io)?,
        Value::String(string) => {
            serde_json::to_writer(writer, string).map_err(HashError::Render)?;
        }
        Value::Array(values) => {
            writer.write_all(b"[").map_err(HashError::Io)?;
            for (index, entry) in values.iter().enumerate() {
                if index > 0 {
                    writer.write_all(b",").map_err(HashError::Io)?;
                }
                write_canonical_json(writer, entry)?;
            }
            writer.write_all(b"]").map_err(HashError::Io)?;
        }
        Value::Object(map) => {
            writer.write_all(b"{").map_err(HashError::Io)?;
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    writer.write_all(b",").map_err(HashError::Io)?;
                }
                serde_json::to_writer(&mut *writer, key).map_err(HashError::Render)?;
                writer.write_all(b":").map_err(HashError::Io)?;
                write_canonical_json(writer, &map[*key])?;
            }
            writer.write_all(b"}").map_err(HashError::Io)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, Number, Value, json};

    #[test]
    fn canonical_json_sorts_object_keys() {
        let mut map = Map::new();
        map.insert("z".to_string(), json!(1));
        map.insert("a".to_string(), json!(2));

        let bytes = canonical_json_bytes(&Value::Object(map)).unwrap();
        assert_eq!(String::from_utf8(bytes).unwrap(), r#"{"a":2,"z":1}"#);
    }

    #[test]
    fn canonical_json_is_stable_across_insertion_order() {
        let mut left = Map::new();
        left.insert("z".to_string(), json!([3, 2, 1]));
        left.insert("a".to_string(), json!({"b": true, "a": false}));

        let mut right = Map::new();
        right.insert("a".to_string(), json!({"a": false, "b": true}));
        right.insert("z".to_string(), json!([3, 2, 1]));

        let left_bytes = canonical_json_bytes(&Value::Object(left)).unwrap();
        let right_bytes = canonical_json_bytes(&Value::Object(right)).unwrap();
        assert_eq!(left_bytes, right_bytes);
    }

    #[test]
    fn canonical_json_normalizes_numbers() {
        let value = Value::Number(Number::from_f64(1.0).unwrap());
        let bytes = canonical_json_bytes(&value).unwrap();
        assert_eq!(String::from_utf8(bytes).unwrap(), "1.0");
    }

    #[test]
    fn digest_bytes_matches_known_vectors() {
        let digests = digest_bytes(b"bil-kernel");
        assert_eq!(
            digests.sha256,
            "66c4a70607cc34b4055a36f682a966333dfe1b18d7d895d51f98e533fe9a9dfb"
        );
        assert_eq!(
            digests.blake3,
            "c3ff042db1981612fae3d0057efb1c2a7e8d2299e4fc2564aa47efe882f643a9"
        );
    }

    #[test]
    fn canonical_json_hash_differs_from_pretty_json_hash() {
        let value = json!({"b": 2, "a": 1});
        let canonical = canonical_json_bytes(&value).unwrap();
        let pretty = serde_json::to_vec_pretty(&value).unwrap();

        assert_ne!(canonical, pretty);
        assert_ne!(digest_bytes(&canonical), digest_bytes(&pretty));
    }

    #[test]
    fn canonical_json_slice_rewrites_equivalent_input() {
        let bytes = canonical_json_slice(br#"{"b":2,"a":[3,{"d":4,"c":5}]}"#).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            r#"{"a":[3,{"c":5,"d":4}],"b":2}"#
        );
    }
}
