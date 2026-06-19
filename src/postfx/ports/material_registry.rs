// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `IMaterialRegistry` — hexagonal port for post-fx material / shader-asset lookup.
//!
//! Each post-fx pass needs a shader-keyword-loaded material. This port abstracts
//! the asset-loading backend (in-memory, file, addressables) so the pass can
//! stay engine-agnostic.
//!
//! Reference: `phenotype-voxel/src/ports/material.rs` (Rust port pattern).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Logical classification of a managed post-fx material asset.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PostFxMaterialKind {
    /// Generic copy / passthrough material (used by several passes).
    Copy,
    /// Bloom prefilter / downsample / upscale material.
    Bloom,
    /// Color grading (LUT apply) material.
    ColorGrade,
    /// Ambient-occlusion / SSAO composite material.
    Ssao,
    /// Tonemap / ACES material.
    Tonemap,
    /// Anything not covered above — escape hatch.
    Other,
}

impl Default for PostFxMaterialKind {
    fn default() -> Self {
        PostFxMaterialKind::Copy
    }
}

/// Metadata for a single managed post-fx material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostFxMaterialInfo {
    /// Stable id of this material (e.g. `"bloom-prefilter-v1"`).
    pub id: String,
    /// Logical kind of this material.
    pub kind: PostFxMaterialKind,
    /// Addressable key, AssetBundle path, or Resources path.
    pub asset_path: String,
    /// Required shader-keyword variants that must be enabled.
    pub required_keywords: Vec<String>,
}

impl PostFxMaterialInfo {
    /// Build a new material info.
    pub fn new(
        id: impl Into<String>,
        kind: PostFxMaterialKind,
        asset_path: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            asset_path: asset_path.into(),
            required_keywords: Vec::new(),
        }
    }

    /// Builder-style: add a required keyword.
    pub fn with_keyword(mut self, kw: impl Into<String>) -> Self {
        self.required_keywords.push(kw.into());
        self
    }
}

/// Hexagonal port: post-fx material / asset registry.
///
/// Adapters include [`InMemoryPostFxMaterialRegistry`] (canonical null
/// adapter) and the recording mock used by domain tests.
pub trait PostFxMaterialRegistry {
    /// Returns all materials currently registered.
    fn list(&mut self) -> Vec<PostFxMaterialInfo>;
    /// Looks up a material by id.
    fn find(&mut self, id: &str) -> Option<PostFxMaterialInfo>;
    /// Registers a material. If an entry with the same id already exists, it
    /// is replaced.
    fn register(&mut self, info: PostFxMaterialInfo);
    /// Removes a material by id.
    fn unregister(&mut self, id: &str) -> bool;
}

/// Default in-memory adapter for [`PostFxMaterialRegistry`].
#[derive(Debug, Default, Clone)]
pub struct InMemoryPostFxMaterialRegistry {
    by_id: HashMap<String, PostFxMaterialInfo>,
}

impl InMemoryPostFxMaterialRegistry {
    /// New empty registry.
    pub fn new() -> Self {
        Self::default()
    }
}

impl PostFxMaterialRegistry for InMemoryPostFxMaterialRegistry {
    fn list(&mut self) -> Vec<PostFxMaterialInfo> {
        self.by_id.values().cloned().collect()
    }

    fn find(&mut self, id: &str) -> Option<PostFxMaterialInfo> {
        self.by_id.get(id).cloned()
    }

    fn register(&mut self, info: PostFxMaterialInfo) {
        self.by_id.insert(info.id.clone(), info);
    }

    fn unregister(&mut self, id: &str) -> bool {
        self.by_id.remove(id).is_some()
    }
}

/// Recording mock used by domain tests to assert on registry interaction
/// order.  Each operation is logged to a list the test can replay.
#[derive(Debug, Default, Clone)]
pub struct RecordingPostFxMaterialRegistry {
    by_id: HashMap<String, PostFxMaterialInfo>,
    calls: Vec<String>,
}

impl RecordingPostFxMaterialRegistry {
    /// New empty recording registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the sequence of method names invoked on this mock.
    pub fn calls(&self) -> &[String] {
        &self.calls
    }

    /// Resets the call log (keeps the registry contents intact).
    pub fn reset_calls(&mut self) {
        self.calls.clear();
    }
}

impl PostFxMaterialRegistry for RecordingPostFxMaterialRegistry {
    fn list(&mut self) -> Vec<PostFxMaterialInfo> {
        self.calls.push("list".into());
        self.by_id.values().cloned().collect()
    }

    fn find(&mut self, id: &str) -> Option<PostFxMaterialInfo> {
        self.calls.push(format!("find({id})"));
        self.by_id.get(id).cloned()
    }

    fn register(&mut self, info: PostFxMaterialInfo) {
        self.calls.push(format!("register({})", info.id));
        self.by_id.insert(info.id.clone(), info);
    }

    fn unregister(&mut self, id: &str) -> bool {
        self.calls.push(format!("unregister({id})"));
        self.by_id.remove(id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_register_and_find_roundtrip() {
        let mut reg = InMemoryPostFxMaterialRegistry::new();
        reg.register(
            PostFxMaterialInfo::new(
                "bloom-prefilter-v1",
                PostFxMaterialKind::Bloom,
                "Shaders/PostFx/Bloom/Prefilter",
            )
            .with_keyword("_BLOOM_HQ"),
        );
        reg.register(PostFxMaterialInfo::new(
            "tonemap-aces-v1",
            PostFxMaterialKind::Tonemap,
            "Shaders/PostFx/TonemapAces",
        ));

        let found = reg.find("bloom-prefilter-v1").unwrap();
        assert_eq!(found.kind, PostFxMaterialKind::Bloom);
        assert_eq!(found.required_keywords, vec!["_BLOOM_HQ".to_string()]);
        assert!(reg.find("does-not-exist").is_none());
    }

    #[test]
    fn in_memory_unregister_removes_entry() {
        let mut reg = InMemoryPostFxMaterialRegistry::new();
        reg.register(PostFxMaterialInfo::new(
            "tonemap-aces-v1",
            PostFxMaterialKind::Tonemap,
            "Shaders/PostFx/TonemapAces",
        ));
        assert!(reg.unregister("tonemap-aces-v1"));
        assert!(!reg.unregister("tonemap-aces-v1"));
    }

    #[test]
    fn recording_mock_captures_call_sequence() {
        let mut reg = RecordingPostFxMaterialRegistry::new();
        reg.register(PostFxMaterialInfo::new(
            "bloom-prefilter-v1",
            PostFxMaterialKind::Bloom,
            "Shaders/PostFx/Bloom/Prefilter",
        ));
        reg.find("bloom-prefilter-v1");
        reg.unregister("bloom-prefilter-v1");

        assert_eq!(
            reg.calls(),
            &[
                "register(bloom-prefilter-v1)".to_string(),
                "find(bloom-prefilter-v1)".to_string(),
                "unregister(bloom-prefilter-v1)".to_string(),
            ]
        );
    }
}
