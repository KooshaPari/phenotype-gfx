// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! Compute shader framework for GPU-accelerated voxel processing.
//!
//! This module provides a unified abstraction over WGSL compute shaders used for
//! bulk voxel operations (fill, carve, smooth), parallel mesh generation, and
//! GPU-friendly radix sorting. The Rust core is engine-agnostic (ADR-004);
//! consumers pass the source string to the engine-side compute pipeline.
//!
//! ## Architecture
//!
//! The [`ComputeShader`] trait defines the contract every compute shader must
//! satisfy: returning WGSL source, a dispatch configuration, and bind/unbind
//! lifecycle hooks. Concrete implementations live in sub-modules:
//!
//! - [`voxel_processor`] — bulk voxel fill, carve, and smooth operations.
//! - [`mesh_generator`] — parallel mesh vertex/index generation from voxels.
//! - [`sorting`] — GPU radix sort for index buffers and spatial data.
//!
//! ## Dispatch model
//!
//! Each shader declares a [`DispatchConfig`] describing the workgroup size and
//! the formula for computing the dispatch count from input dimensions. The
//! engine-side pipeline uses this to issue the correct `dispatchWorkgroups`
//! call.

pub mod mesh_generator;
pub mod sorting;
pub mod voxel_processor;

use serde::{Deserialize, Serialize};

/// Workgroup dimensions for a compute dispatch.
///
/// GPU compute shaders execute in fixed-size workgroups; this struct captures
/// the three-axis size that the engine passes to `dispatchWorkgroups(x, y, z)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkgroupSize {
    /// Number of threads along the X axis.
    pub x: u32,
    /// Number of threads along the Y axis.
    pub y: u32,
    /// Number of threads along the Z axis.
    pub z: u32,
}

impl Default for WorkgroupSize {
    fn default() -> Self {
        Self { x: 64, y: 1, z: 1 }
    }
}

impl WorkgroupSize {
    /// Create a 1-D workgroup of the given size.
    pub fn one_d(x: u32) -> Self {
        Self { x, y: 1, z: 1 }
    }

    /// Create a 2-D workgroup.
    pub fn two_d(x: u32, y: u32) -> Self {
        Self { x, y, z: 1 }
    }

    /// Create a 3-D workgroup.
    pub fn three_d(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Total number of threads per workgroup.
    pub fn total_threads(&self) -> u32 {
        self.x * self.y * self.z
    }
}

/// Dispatch configuration describing how a compute shader should be launched.
///
/// The engine uses `workgroup_size` to compute the number of workgroups needed
/// to cover `element_count` elements (ceiling division per axis).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DispatchConfig {
    /// Workgroup thread dimensions.
    pub workgroup_size: WorkgroupSize,
    /// Total number of elements to process.
    pub element_count: u32,
    /// Optional label for debugging / GPU timing markers.
    pub label: Option<String>,
}

impl DispatchConfig {
    /// Create a new dispatch config for a 1-D workload.
    pub fn one_d(element_count: u32, workgroup_size: u32) -> Self {
        Self {
            workgroup_size: WorkgroupSize::one_d(workgroup_size),
            element_count,
            label: None,
        }
    }

    /// Compute the number of workgroups needed along each axis.
    ///
    /// Returns `(x, y, z)` suitable for `dispatchWorkgroups(x, y, z)`.
    pub fn dispatch_count(&self) -> (u32, u32, u32) {
        let ws = &self.workgroup_size;
        let ceil_div = |total: u32, group: u32| -> u32 {
            if group == 0 {
                0
            } else {
                total.div_ceil(group)
            }
        };
        let count_1d = ceil_div(self.element_count, ws.x);
        (count_1d, 1, 1)
    }

    /// Attach a debug label to this config.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// A compute pipeline binding a WGSL shader to its dispatch configuration.
///
/// Consumers call [`ComputePipeline::source`] to retrieve the WGSL string and
/// [`ComputePipeline::dispatch_config`] for the launch parameters, then hand
/// both to the engine-side pipeline builder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputePipeline {
    /// Debug name for the pipeline (used in GPU debuggers / timing markers).
    pub name: String,
    /// Dispatch configuration for this pipeline.
    pub config: DispatchConfig,
}

impl ComputePipeline {
    /// Create a new compute pipeline.
    pub fn new(name: impl Into<String>, config: DispatchConfig) -> Self {
        Self {
            name: name.into(),
            config,
        }
    }

    /// Return the dispatch count `(x, y, z)` for `dispatchWorkgroups`.
    pub fn dispatch_count(&self) -> (u32, u32, u32) {
        self.config.dispatch_count()
    }
}

/// Trait implemented by all compute shaders in this framework.
///
/// Each implementor provides:
/// - A WGSL source string (the actual GPU program).
/// - A [`DispatchConfig`] describing how to launch the shader.
/// - `bind`/`unbind` lifecycle hooks for the engine-side pipeline.
pub trait ComputeShader {
    /// Return the WGSL source code for this compute shader.
    fn source(&self) -> &'static str;

    /// Return the dispatch configuration for this shader.
    fn dispatch_config(&self) -> DispatchConfig;

    /// Bind this compute shader to the current command encoder.
    ///
    /// Engine-side: set compute pipeline, bind storage buffers, textures, etc.
    fn bind(&self) {}

    /// Unbind this compute shader from the current command encoder.
    fn unbind(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workgroup_size_defaults() {
        let ws = WorkgroupSize::default();
        assert_eq!(ws.x, 64);
        assert_eq!(ws.y, 1);
        assert_eq!(ws.z, 1);
        assert_eq!(ws.total_threads(), 64);
    }

    #[test]
    fn workgroup_size_constructors() {
        let one = WorkgroupSize::one_d(128);
        assert_eq!(one, WorkgroupSize { x: 128, y: 1, z: 1 });
        assert_eq!(one.total_threads(), 128);

        let two = WorkgroupSize::two_d(16, 16);
        assert_eq!(two.total_threads(), 256);

        let three = WorkgroupSize::three_d(8, 8, 8);
        assert_eq!(three.total_threads(), 512);
    }

    #[test]
    fn dispatch_config_dispatch_count() {
        let cfg = DispatchConfig::one_d(1000, 64);
        // ceil(1000 / 64) = 16
        assert_eq!(cfg.dispatch_count(), (16, 1, 1));
    }

    #[test]
    fn dispatch_config_exact_multiple() {
        let cfg = DispatchConfig::one_d(128, 64);
        assert_eq!(cfg.dispatch_count(), (2, 1, 1));
    }

    #[test]
    fn dispatch_config_with_label() {
        let cfg = DispatchConfig::one_d(256, 64)
            .with_label("voxel-fill");
        assert_eq!(cfg.label.as_deref(), Some("voxel-fill"));
    }

    #[test]
    fn compute_pipeline_dispatch_count() {
        let cfg = DispatchConfig::one_d(1024, 64).with_label("test");
        let pipeline = ComputePipeline::new("test-pipeline", cfg);
        assert_eq!(pipeline.dispatch_count(), (16, 1, 1));
    }
}
