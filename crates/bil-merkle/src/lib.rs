use bil_core::{
    BundleManifest, DigestAlgorithm, ManifestEntry, MerkleDocument, MerkleLeaf, MerkleLevel,
    MerkleTreeDocument, MerkleTrees, SCHEMA_VERSION_V0,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MerkleError {
    #[error("manifest must contain at least one evidence entry")]
    EmptyManifest,
    #[error("manifest validation failed: {0}")]
    Core(#[from] bil_core::CoreError),
    #[error("invalid hex digest for {logical_path} using {algorithm}: {source}")]
    InvalidDigest {
        logical_path: String,
        algorithm: DigestAlgorithm,
        #[source]
        source: hex::FromHexError,
    },
    #[error("stored merkle document does not match the manifest")]
    TreeMismatch,
}

pub fn build_manifest_tree(manifest: &BundleManifest) -> Result<MerkleDocument, MerkleError> {
    let manifest = manifest.normalized()?;

    if manifest.entries.is_empty() {
        return Err(MerkleError::EmptyManifest);
    }

    let leaves = manifest
        .entries
        .iter()
        .map(|entry| MerkleLeaf {
            logical_path: entry.logical_path.clone(),
            digests: entry.digests.clone(),
        })
        .collect::<Vec<_>>();

    let leaf_order = manifest
        .entries
        .iter()
        .map(|entry| entry.logical_path.clone())
        .collect::<Vec<_>>();

    let sha256_tree = build_tree(&manifest.entries, DigestAlgorithm::Sha256)?;
    let blake3_tree = build_tree(&manifest.entries, DigestAlgorithm::Blake3)?;

    Ok(MerkleDocument {
        schema_version: SCHEMA_VERSION_V0.to_string(),
        leaf_order,
        leaves,
        trees: MerkleTrees {
            sha256: sha256_tree,
            blake3: blake3_tree,
        },
    })
}

pub fn verify_manifest_tree(
    manifest: &BundleManifest,
    merkle: &MerkleDocument,
) -> Result<(), MerkleError> {
    let rebuilt = build_manifest_tree(manifest)?;
    if rebuilt == *merkle {
        Ok(())
    } else {
        Err(MerkleError::TreeMismatch)
    }
}

fn build_tree(
    entries: &[ManifestEntry],
    algorithm: DigestAlgorithm,
) -> Result<MerkleTreeDocument, MerkleError> {
    let mut levels = Vec::new();
    let mut current = entries
        .iter()
        .map(|entry| decode_leaf(entry, algorithm))
        .collect::<Result<Vec<_>, _>>()?;

    levels.push(MerkleLevel {
        level: 0,
        nodes: current.iter().map(hex::encode).collect(),
    });

    let mut level_index = 1;
    while current.len() > 1 {
        if current.len() % 2 == 1 {
            let duplicate = current.last().cloned().expect("non-empty level");
            current.push(duplicate);
        }

        let next = current
            .chunks(2)
            .map(|pair| match algorithm {
                DigestAlgorithm::Sha256 => hash_pair_sha256(&pair[0], &pair[1]),
                DigestAlgorithm::Blake3 => hash_pair_blake3(&pair[0], &pair[1]),
            })
            .collect::<Vec<_>>();

        levels.push(MerkleLevel {
            level: level_index,
            nodes: next.iter().map(hex::encode).collect(),
        });

        current = next;
        level_index += 1;
    }

    let root = hex::encode(&current[0]);
    Ok(MerkleTreeDocument {
        algorithm,
        root,
        levels,
    })
}

fn decode_leaf(entry: &ManifestEntry, algorithm: DigestAlgorithm) -> Result<Vec<u8>, MerkleError> {
    let value = match algorithm {
        DigestAlgorithm::Sha256 => &entry.digests.sha256,
        DigestAlgorithm::Blake3 => &entry.digests.blake3,
    };

    hex::decode(value).map_err(|source| MerkleError::InvalidDigest {
        logical_path: entry.logical_path.clone(),
        algorithm,
        source,
    })
}

fn hash_pair_sha256(left: &[u8], right: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().to_vec()
}

fn hash_pair_blake3(left: &[u8], right: &[u8]) -> Vec<u8> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().as_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bil_core::{CanonicalizationMode, DigestSet};

    fn manifest(entries: Vec<ManifestEntry>) -> BundleManifest {
        BundleManifest {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            entries,
        }
    }

    fn entry(path: &str, sha256: &str, blake3: &str) -> ManifestEntry {
        ManifestEntry {
            logical_path: path.to_string(),
            media_type: "application/json".to_string(),
            canonicalization: CanonicalizationMode::JsonCanonicalV0,
            byte_length: 1,
            digests: DigestSet {
                sha256: sha256.to_string(),
                blake3: blake3.to_string(),
            },
        }
    }

    #[test]
    fn merkle_tree_sorts_leaves_by_path() {
        let manifest = manifest(vec![
            entry(
                "z.json",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            ),
            entry(
                "a.json",
                "1111111111111111111111111111111111111111111111111111111111111111",
                "2222222222222222222222222222222222222222222222222222222222222222",
            ),
        ]);

        let document = build_manifest_tree(&manifest).unwrap();
        assert_eq!(document.leaf_order, vec!["a.json", "z.json"]);
    }

    #[test]
    fn merkle_tree_duplicates_odd_leaf() {
        let manifest = manifest(vec![
            entry(
                "a.json",
                "1111111111111111111111111111111111111111111111111111111111111111",
                "2222222222222222222222222222222222222222222222222222222222222222",
            ),
            entry(
                "b.json",
                "3333333333333333333333333333333333333333333333333333333333333333",
                "4444444444444444444444444444444444444444444444444444444444444444",
            ),
            entry(
                "c.json",
                "5555555555555555555555555555555555555555555555555555555555555555",
                "6666666666666666666666666666666666666666666666666666666666666666",
            ),
        ]);

        let document = build_manifest_tree(&manifest).unwrap();
        assert_eq!(document.trees.sha256.levels[0].nodes.len(), 3);
        assert_eq!(document.trees.sha256.levels[1].nodes.len(), 2);
    }

    #[test]
    fn merkle_roots_are_stable() {
        let manifest = manifest(vec![
            entry(
                "a.json",
                "1111111111111111111111111111111111111111111111111111111111111111",
                "2222222222222222222222222222222222222222222222222222222222222222",
            ),
            entry(
                "b.json",
                "3333333333333333333333333333333333333333333333333333333333333333",
                "4444444444444444444444444444444444444444444444444444444444444444",
            ),
        ]);

        let document = build_manifest_tree(&manifest).unwrap();
        assert_eq!(
            document.trees.sha256.root,
            "b0dcb09af5496e779e60b21109a718475091191efc7a8638b01d51c622fc9128"
        );
        assert_eq!(
            document.trees.blake3.root,
            "49823d244021c1601bb99044fe1012fd3b102f0bc0bb096b6b023ef5033d8718"
        );
    }
}
