//! PBR (physically-based rendering) policy substrate.
//!
//! Pure-Rust, engine-agnostic data layer for PBR material policy:
//! licensing attestation, render-mode selection for distant LOD,
//! canonical-vs-primitive material seed manifests, and the
//! missing-texture policy (loud in dev, flat tint in player builds).
//!
//! Folded from `civis-platform-wt/crates/voxel/src/material_pbr.rs`.
//! Only dependency is [`phenotype_voxel::MaterialId`] + `serde`.
//! No Bevy, no wgpu, no asset loader — engine adapters consume this.
//!
//! FR coverage (Civis origin; portable to any consumer):
//!
//! - [`LicenseAttestation`]                  → CC0 sourcing.
//! - [`RenderMode`] / [`LodRenderPlan`]       → vertex-color fallback for distant LOD.
//! - [`MaterialMode`] / [`MaterialSeedManifest`] → canonical exemplar vs. per-matid override.
//! - [`MissingTexturePolicy`] / [`PolicyAction`] → loud failure in dev, flat-tint in player.
//!
//! See ADR-0001 for why this lives in the single core.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use phenotype_voxel::MaterialId;

// ---------------------------------------------------------------------------
// CC0 sourcing
// ---------------------------------------------------------------------------

/// Sources we accept as CC0 for shipped PBR textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cc0Source {
    /// ambientCG (https://ambientcg.com) — CC0 PBR texture sets.
    AmbientCg,
    /// Poly Haven (https://polyhaven.com) — CC0 textures and models.
    PolyHaven,
}

impl Cc0Source {
    /// Stable string slug used in asset manifests and log lines.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Cc0Source::AmbientCg => "ambient_cg",
            Cc0Source::PolyHaven => "poly_haven",
        }
    }
}

/// Errors that [`LicenseAttestation::new`] can return.
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
        let asset_path = asset_path.into();
        let manifest_url = manifest_url.into();
        let attested_by = attested_by.into();
        if manifest_url.trim().is_empty() {
            return Err(AttestationError::EmptyManifestUrl);
        }
        if attested_by.trim().is_empty() {
            return Err(AttestationError::EmptyAttestedBy);
        }
        Ok(Self {
            asset_path,
            license: "CC0".to_string(),
            source,
            manifest_url,
            attested_by,
            content_sha256: None,
        })
    }

    /// Attach a content hash after [`Self::new`].
    #[must_use]
    pub fn with_content_sha256(mut self, hash: impl Into<String>) -> Self {
        self.content_sha256 = Some(hash.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Distant-LOD vertex-color shading
// ---------------------------------------------------------------------------

/// Render mode selected for a given chunk distance from the camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderMode {
    /// Full PBR triplanar shading.
    PbrTriplanar,
    /// PBR atlas sampling.
    PbrAtlas,
    /// Flat vertex-color shading. Used at the outer LOD ring.
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
    /// Sanity-check the thresholds. Returns `Err` if not monotonically non-decreasing.
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
    /// `0.0`–`1.0` blend factor into vertex-color shading.
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
        let vertex_color_blend = match mode {
            RenderMode::PbrTriplanar | RenderMode::PbrAtlas => 0.0,
            RenderMode::VertexColor => 1.0,
        };
        Self {
            mode,
            vertex_color_blend,
        }
    }
}

// ---------------------------------------------------------------------------
// Canonical exemplar vs. primitive per-matid mode
// ---------------------------------------------------------------------------

/// Selects whether the material seed manifest uses a canonical exemplar or
/// allows per-`MaterialId` overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialMode {
    /// Reference an exemplar material seed manifest by stable id.
    Canonical,
    /// Allow per-`MaterialId` overrides on top of the exemplar.
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

/// Errors that [`MaterialSeedManifest::resolve`] can return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// `Canonical` mode received a per-matid override; reject loudly.
    CanonicalRejectsOverrides(MaterialId),
    /// No exemplar or override produced a result for this matid.
    NoEntryForMaterial(MaterialId),
}

/// The full material seed manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialSeedManifest {
    /// Mode flag.
    pub mode: MaterialMode,
    /// Stable id of the exemplar set.
    pub exemplar_id: String,
    /// Optional per-matid overrides. Only consulted in `Primitive` mode.
    pub per_matid_overrides: BTreeMap<MaterialId, MaterialOverride>,
    /// Schema version.
    pub schema_version: String,
}

/// Schema version of [`MaterialSeedManifest`].
pub const SCHEMA_VERSION: &str = "0.1.0-pbr-manifest";

impl MaterialSeedManifest {
    /// Construct a manifest in `Canonical` mode.
    #[must_use]
    pub fn canonical(exemplar_id: impl Into<String>) -> Self {
        Self {
            mode: MaterialMode::Canonical,
            exemplar_id: exemplar_id.into(),
            per_matid_overrides: BTreeMap::new(),
            schema_version: SCHEMA_VERSION.to_string(),
        }
    }

    /// Construct a manifest in `Primitive` mode.
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

// ---------------------------------------------------------------------------
// Missing-texture loud vs. flat-tint degrade
// ---------------------------------------------------------------------------

/// Build flavour the client is currently running in.
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
    /// Choose `Loud` in `Dev` and `FlatTint` in `Player`.
    Auto,
    /// Always panic with a clear error.
    Loud,
    /// Always log a warning and degrade to a flat sRGB tint.
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

/// The runtime action the engine adapter must take on a missing texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    /// Panic with a descriptive error (dev build default).
    Panic,
    /// Log a warning, draw with a flat sRGB tint, and continue.
    FlatTintWithWarning,
}

/// Outcome of a single texture load attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissingTextureReport {
    /// Asset path that was requested.
    pub path: String,
    /// `true` if the loader returned a non-empty handle.
    pub loaded: bool,
    /// Build-time policy selected.
    pub policy: MissingTexturePolicy,
    /// Build flavour the policy resolves against in `Auto` mode.
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

/// Runtime action the engine adapter takes for a single chunk's texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAction {
    /// Texture loaded; no change.
    Keep,
    /// Missing texture in dev: panic.
    Panic,
    /// Missing texture in player: log warning + flat tint.
    FlatTintWithWarning,
}

// ---------------------------------------------------------------------------
// PBR channel wiring (separate MR + AO maps; ORM fan-out)
// ---------------------------------------------------------------------------

/// Which PBR data slot a single texture channel feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PbrChannel {
    /// sRGB albedo / base color.
    Albedo,
    /// Linear tangent-space normal map.
    Normal,
    /// Metallic factor (0.0 dielectric .. 1.0 metal).
    Metallic,
    /// Perceptual roughness (0.0 mirror .. 1.0 matte).
    Roughness,
    /// Ambient occlusion (multiplicative; 1.0 = un-occluded).
    AmbientOcclusion,
}

/// The PBR channel-to-source-map binding for one logical material slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TextureChannelMap {
    /// Asset path for the albedo texture (sRGB on disk).
    pub albedo_path: Option<String>,
    /// Asset path for the tangent-space normal map.
    pub normal_path: Option<String>,
    /// Asset path for the standalone metallic-roughness map (G=rough, B=metal).
    pub mr_path: Option<String>,
    /// Asset path for the standalone AO map (R=AO).
    pub ao_path: Option<String>,
    /// Asset path for the combined ORM map (R=AO, G=Rough, B=Metal).
    pub orm_path: Option<String>,
}

impl TextureChannelMap {
    /// Minimal channel map: albedo + normal only.
    #[must_use]
    pub fn minimal(albedo: impl Into<String>, normal: impl Into<String>) -> Self {
        Self {
            albedo_path: Some(albedo.into()),
            normal_path: Some(normal.into()),
            mr_path: None,
            ao_path: None,
            orm_path: None,
        }
    }

    /// Full standalone map: albedo + normal + dedicated MR + AO.
    #[must_use]
    pub fn standalone(
        albedo: impl Into<String>,
        normal: impl Into<String>,
        mr: impl Into<String>,
        ao: impl Into<String>,
    ) -> Self {
        Self {
            albedo_path: Some(albedo.into()),
            normal_path: Some(normal.into()),
            mr_path: Some(mr.into()),
            ao_path: Some(ao.into()),
            orm_path: None,
        }
    }

    /// ORM-packed map: albedo + normal + a single ORM file.
    #[must_use]
    pub fn orm_packed(
        albedo: impl Into<String>,
        normal: impl Into<String>,
        orm: impl Into<String>,
    ) -> Self {
        Self {
            albedo_path: Some(albedo.into()),
            normal_path: Some(normal.into()),
            mr_path: None,
            ao_path: None,
            orm_path: Some(orm.into()),
        }
    }

    /// Every channel-source path in deterministic order: albedo→normal→mr→ao→orm.
    pub fn required_paths(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::with_capacity(5);
        if let Some(p) = self.albedo_path.as_deref() { out.push(p); }
        if let Some(p) = self.normal_path.as_deref() { out.push(p); }
        if let Some(p) = self.mr_path.as_deref() { out.push(p); }
        if let Some(p) = self.ao_path.as_deref() { out.push(p); }
        if let Some(p) = self.orm_path.as_deref() { out.push(p); }
        out
    }

    /// `true` when the configuration can drive every standard PBR channel.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.albedo_path.is_some()
            && self.normal_path.is_some()
            && (self.mr_path.is_some() || self.orm_path.is_some())
            && (self.ao_path.is_some() || self.orm_path.is_some())
    }
}

// ---------------------------------------------------------------------------
// Color-space policy
// ---------------------------------------------------------------------------

/// Per-slot color-space flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorSpace {
    /// sRGB transfer function on disk; engine stores linear after decode.
    Srgb,
    /// Linear data; engine stores as-is.
    Linear,
}

/// Color-space policy consulted at `load_with_settings` time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorSpacePolicy {
    /// Color space for every albedo / emissive / decal slot.
    pub color: ColorSpace,
    /// Color space for every data slot (normal, metallic, roughness, AO).
    pub data: ColorSpace,
}

impl Default for ColorSpacePolicy {
    fn default() -> Self {
        Self {
            color: ColorSpace::Srgb,
            data: ColorSpace::Linear,
        }
    }
}

impl ColorSpacePolicy {
    /// Spec-mandated policy: color = sRGB, data = linear.
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            color: ColorSpace::Srgb,
            data: ColorSpace::Linear,
        }
    }

    /// Look up the color space for a given channel.
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

// ---------------------------------------------------------------------------
// Triplanar splatting
// ---------------------------------------------------------------------------

/// One layer of a triplanar splat plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriplanarLayer {
    /// Material id this layer binds to.
    pub matid: MaterialId,
    /// Per-channel map for this layer.
    pub channels: TextureChannelMap,
    /// Relative blend weight (0.0..1.0).
    pub splat_weight: f32,
    /// World-space tile size in voxel units.
    pub world_tile: f32,
}

impl TriplanarLayer {
    /// Construct a layer with default tile size.
    #[must_use]
    pub fn new(matid: MaterialId, channels: TextureChannelMap, splat_weight: f32) -> Self {
        Self { matid, channels, splat_weight, world_tile: 1.0 }
    }

    /// Override the world-space tile size.
    #[must_use]
    pub fn with_world_tile(mut self, world_tile: f32) -> Self {
        self.world_tile = world_tile;
        self
    }

    fn is_complete_layer(&self) -> bool {
        self.channels.is_complete() && self.splat_weight.is_finite() && self.splat_weight >= 0.0
    }
}

/// A pure-data triplanar splat plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriplanarSplatPlan {
    /// Per-`MaterialId` layer in deterministic ascending-matid order.
    pub layers: BTreeMap<MaterialId, TriplanarLayer>,
    /// Schema version.
    pub schema_version: String,
}

impl TriplanarSplatPlan {
    /// Construct an empty plan.
    #[must_use]
    pub fn new() -> Self {
        Self {
            layers: BTreeMap::new(),
            schema_version: SCHEMA_VERSION.to_string(),
        }
    }

    /// Insert or replace a layer.
    pub fn insert(&mut self, layer: TriplanarLayer) -> &mut Self {
        self.layers.insert(layer.matid, layer);
        self
    }

    /// Iterate layers in deterministic `matid`-ascending order.
    pub fn iter_ordered(&self) -> impl Iterator<Item = (&MaterialId, &TriplanarLayer)> {
        self.layers.iter()
    }

    /// Normalize splat weights so the maximum equals `1.0`.
    pub fn normalize_weights(&mut self) {
        let max = self
            .layers
            .values()
            .map(|l| l.splat_weight)
            .fold(0.0_f32, f32::max);
        if max > 0.0 {
            for layer in self.layers.values_mut() {
                layer.splat_weight /= max;
            }
        }
    }

    /// `true` if every layer has a complete channel map and at least one layer exists.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.layers.is_empty() && self.layers.values().all(TriplanarLayer::is_complete_layer)
    }
}

impl Default for TriplanarSplatPlan {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Greedy mesh + texture-array atlas
// ---------------------------------------------------------------------------

/// One slice of the texture-array atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AtlasSlice {
    /// Material id this slice represents.
    pub matid: MaterialId,
    /// Zero-based layer index in the 2D array texture.
    pub array_layer: u16,
}

/// Greedy-mesh + texture-array atlas plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GreedyAtlasPlan {
    /// Sorted by `matid` for replay determinism.
    pub slices: BTreeMap<MaterialId, AtlasSlice>,
    /// Total number of array layers used.
    pub array_depth: u16,
    /// Schema version.
    pub schema_version: String,
}

impl GreedyAtlasPlan {
    /// Construct an empty plan.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slices: BTreeMap::new(),
            array_depth: 0,
            schema_version: SCHEMA_VERSION.to_string(),
        }
    }

    /// Assign the next free array layer to `matid`.
    pub fn allocate(&mut self, matid: MaterialId) -> &AtlasSlice {
        let layer = self.array_depth;
        self.array_depth = self.array_depth.saturating_add(1);
        self.slices.insert(matid, AtlasSlice { matid, array_layer: layer });
        self.slices.get(&matid).expect("slice was just inserted")
    }

    /// Look up the array layer for `matid`.
    #[must_use]
    pub fn layer_for(&self, matid: MaterialId) -> Option<u16> {
        self.slices.get(&matid).map(|s| s.array_layer)
    }
}

impl Default for GreedyAtlasPlan {
    fn default() -> Self {
        Self::new()
    }
}
