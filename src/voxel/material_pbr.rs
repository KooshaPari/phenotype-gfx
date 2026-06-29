//! PBR material policy substrate — CC0 attestation, LOD render plan,
//! material seed manifests, missing-texture policy, channel maps,
//! triplanar splat plans, greedy atlas plans.
//!
//! Folded from `civis-platform-wt/crates/voxel/src/material_pbr.rs`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use crate::voxel::MaterialId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// -----------------------------------------------------------------------
// CC0 sourcing
// -----------------------------------------------------------------------

/// Sources we accept as CC0 for shipped PBR textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cc0Source {
    /// ambientCG (https://ambientcg.com).
    AmbientCg,
    /// Poly Haven (https://polyhaven.com).
    PolyHaven,
}

impl Cc0Source {
    /// Stable string slug used in asset manifests.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Cc0Source::AmbientCg => "ambient_cg",
            Cc0Source::PolyHaven => "poly_haven",
        }
    }
}

/// Errors from [`LicenseAttestation::new`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationError {
    /// `manifest_url` was empty.
    EmptyManifestUrl,
    /// `attested_by` was empty.
    EmptyAttestedBy,
}

/// One row of the asset-manifest CC0 attestation table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseAttestation {
    /// Asset path inside the build.
    pub asset_path: String,
    /// License string. Always `"CC0"`.
    pub license: String,
    /// Enumerated CC0 source.
    pub source: Cc0Source,
    /// URL to the original distribution manifest entry.
    pub manifest_url: String,
    /// Human or build-pipeline id that added this attestation.
    pub attested_by: String,
    /// Optional content-hash for tamper detection.
    pub content_sha256: Option<String>,
}

impl LicenseAttestation {
    /// Construct a [`LicenseAttestation`] after validating required invariants.
    pub fn new(
        asset_path: impl Into<String>,
        source: Cc0Source,
        manifest_url: impl Into<String>,
        attested_by: impl Into<String>,
    ) -> Result<Self, AttestationError> {
        let manifest_url = manifest_url.into();
        let attested_by = attested_by.into();
        if manifest_url.trim().is_empty() {
            return Err(AttestationError::EmptyManifestUrl);
        }
        if attested_by.trim().is_empty() {
            return Err(AttestationError::EmptyAttestedBy);
        }
        Ok(Self {
            asset_path: asset_path.into(),
            license: "CC0".to_string(),
            source,
            manifest_url,
            attested_by,
            content_sha256: None,
        })
    }

    /// Attach a content hash.
    #[must_use]
    pub fn with_content_sha256(mut self, hash: impl Into<String>) -> Self {
        self.content_sha256 = Some(hash.into());
        self
    }
}

// -----------------------------------------------------------------------
// Distant-LOD render mode
// -----------------------------------------------------------------------

/// Render mode selected for a given chunk distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderMode {
    /// Full PBR triplanar shading.
    PbrTriplanar,
    /// PBR atlas sampling.
    PbrAtlas,
    /// Flat vertex-color shading (outer LOD ring).
    VertexColor,
}

/// Distance thresholds (in chunk units) for the render-mode switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LodDistanceConfig {
    /// Distance below which we keep `PbrTriplanar` (inclusive).
    pub near_chunks: u32,
    /// Distance at which we switch to `PbrAtlas` (inclusive).
    pub mid_chunks: u32,
    /// Distance at which we fall back to `VertexColor` (inclusive).
    pub far_chunks: u32,
}

impl Default for LodDistanceConfig {
    fn default() -> Self {
        Self {
            near_chunks: 2,
            mid_chunks: 4,
            far_chunks: 8,
        }
    }
}

impl LodDistanceConfig {
    /// Validate monotonic ordering. Returns `Err` if not monotonically non-decreasing.
    pub fn validate(self) -> Result<(), &'static str> {
        if self.near_chunks > self.mid_chunks {
            return Err("near_chunks must be <= mid_chunks");
        }
        if self.mid_chunks > self.far_chunks {
            return Err("mid_chunks must be <= far_chunks");
        }
        Ok(())
    }
}

/// Output of [`LodRenderPlan::for_distance`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LodRenderPlan {
    /// Selected render mode.
    pub mode: RenderMode,
    /// Blend factor into vertex-color shading (0.0 = full PBR, 1.0 = full VC).
    pub vertex_color_blend: f32,
}

impl LodRenderPlan {
    /// Decide the render mode for a chunk at `distance_chunks` ring distance.
    pub fn for_distance(distance_chunks: u32, config: LodDistanceConfig) -> Self {
        let mode = if distance_chunks <= config.near_chunks {
            RenderMode::PbrTriplanar
        } else if distance_chunks <= config.mid_chunks {
            RenderMode::PbrAtlas
        } else {
            RenderMode::VertexColor
        };
        let vertex_color_blend = if matches!(mode, RenderMode::VertexColor) {
            1.0
        } else {
            0.0
        };
        Self {
            mode,
            vertex_color_blend,
        }
    }
}

// -----------------------------------------------------------------------
// Material seed manifest
// -----------------------------------------------------------------------

/// Canonical exemplar vs. per-matid override mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialMode {
    /// All matids resolve through the exemplar; no per-matid overrides.
    Canonical,
    /// Per-matid overrides layer on top of the exemplar.
    Primitive,
}

/// A single per-`MaterialId` override used in `Primitive` mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MaterialOverride {
    /// Optional albedo path override.
    pub albedo_path: Option<String>,
    /// Optional normal-map override.
    pub normal_path: Option<String>,
    /// Optional ORM override.
    pub orm_path: Option<String>,
    /// Per-matid roughness override (0.0–1.0).
    pub perceptual_roughness: Option<f32>,
    /// Per-matid tint multiplied with the exemplar albedo. RGB in sRGB, stored as `f32`.
    pub tint_srgb: Option<[f32; 3]>,
}

/// Errors from [`MaterialSeedManifest::resolve`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// Canonical mode received a per-matid override.
    CanonicalRejectsOverrides(MaterialId),
    /// No exemplar or override produced a result.
    NoEntryForMaterial(MaterialId),
}

/// Schema version of [`MaterialSeedManifest`].
pub const SCHEMA_VERSION: &str = "0.1.0-pbr-manifest";

/// The full material seed manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialSeedManifest {
    /// Mode flag.
    pub mode: MaterialMode,
    /// Stable id of the exemplar set.
    pub exemplar_id: String,
    /// Per-matid overrides (sorted by `MaterialId` for determinism).
    pub per_matid_overrides: BTreeMap<MaterialId, MaterialOverride>,
    /// Schema version.
    pub schema_version: String,
}

impl MaterialSeedManifest {
    /// Construct in `Canonical` mode.
    #[must_use]
    pub fn canonical(exemplar_id: impl Into<String>) -> Self {
        Self {
            mode: MaterialMode::Canonical,
            exemplar_id: exemplar_id.into(),
            per_matid_overrides: BTreeMap::new(),
            schema_version: SCHEMA_VERSION.to_string(),
        }
    }

    /// Construct in `Primitive` mode.
    #[must_use]
    pub fn primitive(
        exemplar_id: impl Into<String>,
        per_matid_overrides: BTreeMap<MaterialId, MaterialOverride>,
    ) -> Self {
        Self {
            mode: MaterialMode::Primitive,
            exemplar_id: exemplar_id.into(),
            per_matid_overrides,
            schema_version: SCHEMA_VERSION.to_string(),
        }
    }

    /// Resolve a single matid to its override (if any).
    pub fn resolve(&self, matid: MaterialId) -> Result<Option<&MaterialOverride>, ManifestError> {
        match self.mode {
            MaterialMode::Canonical => {
                if self.per_matid_overrides.contains_key(&matid) {
                    Err(ManifestError::CanonicalRejectsOverrides(matid))
                } else {
                    Ok(None)
                }
            }
            MaterialMode::Primitive => match self.per_matid_overrides.get(&matid) {
                Some(ov) => Ok(Some(ov)),
                None => Err(ManifestError::NoEntryForMaterial(matid)),
            },
        }
    }
}

// -----------------------------------------------------------------------
// Missing-texture policy
// -----------------------------------------------------------------------

/// Build flavour for missing-texture policy resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildFlavour {
    /// Developer build — missing textures MUST fail loud.
    Dev,
    /// Player/shipping build — missing textures MUST degrade to flat tint.
    Player,
}

/// Build-time selectable missing-texture policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingTexturePolicy {
    /// Choose `Loud` in dev, `FlatTint` in player.
    Auto,
    /// Always panic.
    Loud,
    /// Always log warning + flat tint.
    FlatTint,
}

impl MissingTexturePolicy {
    /// Resolve `Auto` against a [`BuildFlavour`].
    #[must_use]
    pub fn resolve(self, flavour: BuildFlavour) -> PolicyAction {
        match self {
            MissingTexturePolicy::Auto => match flavour {
                BuildFlavour::Dev => PolicyAction::Panic,
                BuildFlavour::Player => PolicyAction::FlatTintWithWarning,
            },
            MissingTexturePolicy::Loud => PolicyAction::Panic,
            MissingTexturePolicy::FlatTint => PolicyAction::FlatTintWithWarning,
        }
    }
}

/// The runtime action for a missing texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    /// Panic with a descriptive error.
    Panic,
    /// Log a warning, draw with flat tint, continue.
    FlatTintWithWarning,
}

/// Outcome of a single texture load attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissingTextureReport {
    /// Asset path requested.
    pub path: String,
    /// `true` if the loader returned a handle.
    pub loaded: bool,
    /// Policy selected.
    pub policy: MissingTexturePolicy,
    /// Build flavour.
    pub flavour: BuildFlavour,
}

impl MissingTextureReport {
    /// Compute the runtime action for this report.
    #[must_use]
    pub fn action(&self) -> RuntimeAction {
        if self.loaded {
            return RuntimeAction::Keep;
        }
        match self.policy.resolve(self.flavour) {
            PolicyAction::Panic => RuntimeAction::Panic,
            PolicyAction::FlatTintWithWarning => RuntimeAction::FlatTintWithWarning,
        }
    }
}

/// Runtime action the engine adapter takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAction {
    /// Texture loaded; no change.
    Keep,
    /// Missing in dev: panic.
    Panic,
    /// Missing in player: log + flat tint.
    FlatTintWithWarning,
}

// -----------------------------------------------------------------------
// PBR channel wiring
// -----------------------------------------------------------------------

/// Which PBR slot a single texture channel feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PbrChannel {
    /// sRGB albedo / base color.
    Albedo,
    /// Linear tangent-space normal map.
    Normal,
    /// Metallic factor.
    Metallic,
    /// Perceptual roughness.
    Roughness,
    /// Ambient occlusion.
    AmbientOcclusion,
}

/// PBR channel-to-source-map binding for one logical material slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TextureChannelMap {
    /// Albedo texture path (sRGB on disk).
    pub albedo_path: Option<String>,
    /// Tangent-space normal map path.
    pub normal_path: Option<String>,
    /// Standalone metallic-roughness map (G=rough, B=metal).
    pub mr_path: Option<String>,
    /// Standalone AO map (R=AO).
    pub ao_path: Option<String>,
    /// Combined ORM map (R=AO, G=Rough, B=Metal).
    pub orm_path: Option<String>,
}

impl TextureChannelMap {
    /// Minimal map: albedo + normal only.
    #[must_use]
    pub fn minimal(albedo: impl Into<String>, normal: impl Into<String>) -> Self {
        Self {
            albedo_path: Some(albedo.into()),
            normal_path: Some(normal.into()),
            ..Default::default()
        }
    }

    /// Full standalone map: albedo + normal + dedicated MR + AO.
    #[must_use]
    pub fn standalone(
        a: impl Into<String>,
        n: impl Into<String>,
        mr: impl Into<String>,
        ao: impl Into<String>,
    ) -> Self {
        Self {
            albedo_path: Some(a.into()),
            normal_path: Some(n.into()),
            mr_path: Some(mr.into()),
            ao_path: Some(ao.into()),
            orm_path: None,
        }
    }

    /// ORM-packed map: albedo + normal + single ORM file.
    #[must_use]
    pub fn orm_packed(a: impl Into<String>, n: impl Into<String>, orm: impl Into<String>) -> Self {
        Self {
            albedo_path: Some(a.into()),
            normal_path: Some(n.into()),
            orm_path: Some(orm.into()),
            ..Default::default()
        }
    }

    /// All channel-source paths in deterministic order: albedo→normal→mr→ao→orm.
    pub fn required_paths(&self) -> Vec<&str> {
        let mut out = Vec::with_capacity(5);
        for p in [
            &self.albedo_path,
            &self.normal_path,
            &self.mr_path,
            &self.ao_path,
            &self.orm_path,
        ] {
            if let Some(s) = p {
                out.push(s.as_str());
            }
        }
        out
    }

    /// `true` when the map can drive every standard PBR channel.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.albedo_path.is_some()
            && self.normal_path.is_some()
            && (self.mr_path.is_some() || self.orm_path.is_some())
            && (self.ao_path.is_some() || self.orm_path.is_some())
    }
}

// -----------------------------------------------------------------------
// Color-space policy
// -----------------------------------------------------------------------

/// Per-slot color-space flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorSpace {
    /// sRGB on disk; engine stores linear after decode.
    Srgb,
    /// Linear data; engine stores as-is.
    Linear,
}

/// Color-space policy consulted at load time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorSpacePolicy {
    /// Color space for albedo / emissive / decal slots.
    pub color: ColorSpace,
    /// Color space for data slots (normal, metallic, roughness, AO).
    pub data: ColorSpace,
}

impl Default for ColorSpacePolicy {
    fn default() -> Self {
        Self::strict()
    }
}

impl ColorSpacePolicy {
    /// Spec-mandated: color = sRGB, data = linear.
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            color: ColorSpace::Srgb,
            data: ColorSpace::Linear,
        }
    }

    /// Color space for a given channel.
    #[must_use]
    pub const fn for_channel(&self, channel: PbrChannel) -> ColorSpace {
        match channel {
            PbrChannel::Albedo => self.color,
            PbrChannel::Normal
            | PbrChannel::Metallic
            | PbrChannel::Roughness
            | PbrChannel::AmbientOcclusion => self.data,
        }
    }
}
