use bil_core::{
    AxleEvidenceRecord, BundleDescriptor, BundleManifest, MerkleDocument, ReceiptDocument,
    VerificationReport,
};
use schemars::schema_for;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDocument {
    pub file_name: &'static str,
    pub contents: String,
}

pub fn schema_documents() -> Vec<SchemaDocument> {
    vec![
        SchemaDocument {
            file_name: "axle-evidence-record.schema.json",
            contents: pretty_schema(schema_for!(AxleEvidenceRecord)),
        },
        SchemaDocument {
            file_name: "bundle-descriptor.schema.json",
            contents: pretty_schema(schema_for!(BundleDescriptor)),
        },
        SchemaDocument {
            file_name: "bundle-manifest.schema.json",
            contents: pretty_schema(schema_for!(BundleManifest)),
        },
        SchemaDocument {
            file_name: "merkle-document.schema.json",
            contents: pretty_schema(schema_for!(MerkleDocument)),
        },
        SchemaDocument {
            file_name: "receipt-document.schema.json",
            contents: pretty_schema(schema_for!(ReceiptDocument)),
        },
        SchemaDocument {
            file_name: "verification-report.schema.json",
            contents: pretty_schema(schema_for!(VerificationReport)),
        },
    ]
}

pub fn schema_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/v0")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/v0"))
}

fn pretty_schema<T>(schema: T) -> String
where
    T: serde::Serialize,
{
    let mut contents = serde_json::to_string_pretty(&schema).expect("schema serialization");
    contents.push('\n');
    contents
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn schema_documents_are_current() {
        for document in schema_documents() {
            let path = schema_root().join(document.file_name);
            let committed = fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!("failed to read schema {}: {error}", path.display())
            });
            assert_eq!(
                committed, document.contents,
                "schema {} is stale",
                document.file_name
            );
        }
    }
}
