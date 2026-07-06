use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub type Timings = BTreeMap<String, u64>;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct Messages {
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub infos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct VerifyProofResponse {
    #[serde(default)]
    pub okay: bool,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub lean_messages: Messages,
    #[serde(default)]
    pub tool_messages: Messages,
    #[serde(default)]
    pub failed_declarations: Vec<String>,
    #[serde(default)]
    pub timings: Timings,
    #[serde(default)]
    pub info: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct Document {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub declaration: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tokens: Vec<String>,
    #[serde(default)]
    pub signature: String,
    #[serde(rename = "type", default)]
    pub r#type: String,
    #[serde(default)]
    pub type_hash: u64,
    #[serde(default)]
    pub type_depth: u64,
    #[serde(default)]
    pub term_depth: u64,
    #[serde(default)]
    pub is_sorry: bool,
    #[serde(default)]
    pub index: u64,
    #[serde(default)]
    pub line_pos: u64,
    #[serde(default)]
    pub end_line_pos: u64,
    #[serde(default)]
    pub proof_length: u64,
    #[serde(default)]
    pub tactic_counts: Timings,
    #[serde(default)]
    pub wall_ms: u64,
    #[serde(default)]
    pub heartbeats: u64,
    #[serde(default)]
    pub local_type_dependencies: Vec<String>,
    #[serde(default)]
    pub local_value_dependencies: Vec<String>,
    #[serde(default)]
    pub external_type_dependencies: Vec<String>,
    #[serde(default)]
    pub external_value_dependencies: Vec<String>,
    #[serde(default)]
    pub local_syntactic_dependencies: Vec<String>,
    #[serde(default)]
    pub external_syntactic_dependencies: Vec<String>,
    #[serde(default)]
    pub declaration_messages: Messages,
    #[serde(default)]
    pub theorem_messages: Messages,
}

macro_rules! content_messages_response {
    ($name:ident) => {
        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
        pub struct $name {
            #[serde(default)]
            pub lean_messages: Messages,
            #[serde(default)]
            pub tool_messages: Messages,
            #[serde(default)]
            pub content: String,
            #[serde(default)]
            pub timings: Timings,
            #[serde(default)]
            pub info: Option<Value>,
        }
    };
}

content_messages_response!(RenameResponse);
content_messages_response!(MergeResponse);
content_messages_response!(Theorem2SorryResponse);
content_messages_response!(Theorem2LemmaResponse);
content_messages_response!(Have2SorryResponse);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct CheckResponse {
    #[serde(default)]
    pub okay: bool,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub lean_messages: Messages,
    #[serde(default)]
    pub tool_messages: Messages,
    #[serde(default)]
    pub failed_declarations: Vec<String>,
    #[serde(default)]
    pub timings: Timings,
    #[serde(default)]
    pub info: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct ExtractTheoremsResponse {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub lean_messages: Messages,
    #[serde(default)]
    pub tool_messages: Messages,
    #[serde(default)]
    pub documents: BTreeMap<String, Document>,
    #[serde(default)]
    pub timings: Timings,
    #[serde(default)]
    pub info: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct ExtractDeclsResponse {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub lean_messages: Messages,
    #[serde(default)]
    pub tool_messages: Messages,
    #[serde(default)]
    pub documents: BTreeMap<String, Document>,
    #[serde(default)]
    pub timings: Timings,
    #[serde(default)]
    pub info: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct SimplifyTheoremsResponse {
    #[serde(default)]
    pub lean_messages: Messages,
    #[serde(default)]
    pub tool_messages: Messages,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub timings: Timings,
    #[serde(default)]
    pub simplification_stats: Timings,
    #[serde(default)]
    pub info: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct RepairProofsResponse {
    #[serde(default)]
    pub lean_messages: Messages,
    #[serde(default)]
    pub tool_messages: Messages,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub timings: Timings,
    #[serde(default)]
    pub repair_stats: Timings,
    #[serde(default)]
    pub info: Option<Value>,
    #[serde(default)]
    pub okay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct Have2LemmaResponse {
    #[serde(default)]
    pub lean_messages: Messages,
    #[serde(default)]
    pub tool_messages: Messages,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub lemma_names: Vec<String>,
    #[serde(default)]
    pub timings: Timings,
    #[serde(default)]
    pub info: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct Sorry2LemmaResponse {
    #[serde(default)]
    pub lean_messages: Messages,
    #[serde(default)]
    pub tool_messages: Messages,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub lemma_names: Vec<String>,
    #[serde(default)]
    pub timings: Timings,
    #[serde(default)]
    pub info: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct DisproveResponse {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub lean_messages: Messages,
    #[serde(default)]
    pub tool_messages: Messages,
    #[serde(default)]
    pub results: BTreeMap<String, String>,
    #[serde(default)]
    pub negated: BTreeMap<String, String>,
    #[serde(default)]
    pub disproved_theorems: Vec<String>,
    #[serde(default)]
    pub timings: Timings,
    #[serde(default)]
    pub info: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct NormalizeResponse {
    #[serde(default)]
    pub lean_messages: Messages,
    #[serde(default)]
    pub tool_messages: Messages,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub timings: Timings,
    #[serde(default)]
    pub normalize_stats: Timings,
    #[serde(default)]
    pub info: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn verify_proof_response_roundtrips() {
        let response = VerifyProofResponse {
            okay: true,
            content: "theorem t : 1 = 1 := rfl".into(),
            lean_messages: Messages::default(),
            tool_messages: Messages::default(),
            failed_declarations: vec![],
            timings: BTreeMap::from([("total".into(), 42)]),
            info: Some(json!({"source": "test"})),
        };

        let value = serde_json::to_value(&response).unwrap();
        let decoded: VerifyProofResponse = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, response);
    }

    #[test]
    fn document_roundtrips_with_nested_messages() {
        let document = Document {
            name: "foo".into(),
            kind: "theorem".into(),
            declaration: "theorem foo".into(),
            content: "theorem foo : 1 = 1 := rfl".into(),
            tokens: vec!["theorem".into(), "foo".into()],
            signature: "foo : 1 = 1".into(),
            r#type: "Prop".into(),
            type_hash: 7,
            type_depth: 1,
            term_depth: 1,
            is_sorry: false,
            index: 0,
            line_pos: 1,
            end_line_pos: 1,
            proof_length: 1,
            tactic_counts: BTreeMap::from([("rfl".into(), 1)]),
            wall_ms: 10,
            heartbeats: 20,
            local_type_dependencies: vec!["Nat".into()],
            local_value_dependencies: vec!["rfl".into()],
            external_type_dependencies: vec![],
            external_value_dependencies: vec![],
            local_syntactic_dependencies: vec![],
            external_syntactic_dependencies: vec![],
            declaration_messages: Messages {
                infos: vec!["ok".into()],
                ..Messages::default()
            },
            theorem_messages: Messages::default(),
        };

        let value = serde_json::to_value(&document).unwrap();
        let decoded: Document = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, document);
    }

    #[test]
    fn normalize_response_roundtrips() {
        let response = NormalizeResponse {
            content: "theorem foo : 1 = 1 := rfl".into(),
            normalize_stats: BTreeMap::from([("remove_sections".into(), 2)]),
            ..NormalizeResponse::default()
        };

        let value = serde_json::to_value(&response).unwrap();
        let decoded: NormalizeResponse = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, response);
    }
}
