// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! GPU radix sort for index buffers and spatial data.
//!
//! [`SortingComputeShader`] provides a WGSL compute shader implementing
//! GPU-friendly radix sort. The algorithm processes one radix digit (4 bits)
//! per pass, using local memory for per-workgroup histogramming and a global
//! prefix-sum buffer for scatter offsets.
//!
//! ## Algorithm overview
//!
//! 1. **Histogram**: Each workgroup counts occurrences of each radix value
//!    (0–15) for its chunk of keys.
//! 2. **Prefix sum**: A single-threaded pass (or a separate compute pass in a
//!    full implementation) produces per-workgroup offsets.
//! 3. **Scatter**: Keys and values are written to the output buffer at the
//!    computed offsets.
//!
//! For 32-bit keys this requires 8 passes (4 bits per digit).
//!
//! ## Buffer layout
//!
//! - `@binding(0)` — uniforms (element count, current radix digit, bit shift).
//! - `@binding(1)` — input keys (`array<u32>`).
//! - `@binding(2)` — input values (`array<u32>`).
//! - `@binding(3)` — output keys (`array<u32>`).
//! - `@binding(4)` — output values (`array<u32>`).
//! - `@binding(5)` — histogram buffer (`array<u32>`, 16 × workgroup_count).

use serde::{Deserialize, Serialize};

use super::{ComputeShader, DispatchConfig, WorkgroupSize};

/// Configuration for the radix sort compute shader.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortingConfig {
    /// Number of elements to sort.
    pub element_count: u32,
    /// Number of bits per radix digit (default: 4 for hex radix).
    pub bits_per_digit: u32,
    /// Total number of bits in the key type (default: 32).
    pub key_bits: u32,
}

impl Default for SortingConfig {
    fn default() -> Self {
        Self {
            element_count: 1024,
            bits_per_digit: 4,
            key_bits: 32,
        }
    }
}

impl SortingConfig {
    /// Number of radix digits needed for the full key width.
    pub fn digit_count(&self) -> u32 {
        (self.key_bits + self.bits_per_digit - 1) / self.bits_per_digit
    }

    /// Number of radix values per digit (e.g. 16 for 4-bit digits).
    pub fn radix_size(&self) -> u32 {
        1u32 << self.bits_per_digit
    }

    /// Bit mask for extracting a single digit.
    pub fn digit_mask(&self) -> u32 {
        self.radix_size() - 1
    }

    /// Number of workgroups needed to process all elements.
    pub fn workgroup_count(&self, workgroup_size: u32) -> u32 {
        if workgroup_size == 0 {
            0
        } else {
            (self.element_count + workgroup_size - 1) / workgroup_size
        }
    }

    /// Size of the histogram buffer (radix_size × workgroup_count).
    pub fn histogram_size(&self, workgroup_size: u32) -> u32 {
        self.radix_size() * self.workgroup_count(workgroup_size)
    }
}

/// GPU compute shader implementing radix sort for u32 keys.
///
/// Implements [`ComputeShader`] with an embedded WGSL compute shader that
/// performs one digit pass of radix sort per dispatch. The engine re-dispatches
/// this shader once per radix digit, alternating input/output buffers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortingComputeShader {
    /// Configuration controlling element count and radix parameters.
    pub config: SortingConfig,
}

impl Default for SortingComputeShader {
    fn default() -> Self {
        Self {
            config: SortingConfig::default(),
        }
    }
}

impl SortingComputeShader {
    /// Create a new sorting compute shader with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a sorting shader for the given number of u32 elements.
    pub fn for_count(element_count: u32) -> Self {
        Self {
            config: SortingConfig {
                element_count,
                ..Default::default()
            },
        }
    }

    /// Create a sorting shader with custom radix parameters.
    pub fn with_params(element_count: u32, bits_per_digit: u32, key_bits: u32) -> Self {
        Self {
            config: SortingConfig {
                element_count,
                bits_per_digit,
                key_bits,
            },
        }
    }

}

impl ComputeShader for SortingComputeShader {
    fn source(&self) -> &'static str {
        SORTING_WGSL
    }

    fn dispatch_config(&self) -> DispatchConfig {
        DispatchConfig {
            workgroup_size: WorkgroupSize::one_d(256),
            element_count: self.config.element_count,
            label: None,
        }
    }

    fn bind(&self) {
        // Engine-side: set compute pipeline, bind key/value buffers + histogram.
    }

    fn unbind(&self) {
        // Engine-side: unbind compute pipeline.
    }
}

/// Embedded WGSL source for the radix sort compute shader.
///
/// This shader performs a single pass of radix sort for one digit position.
/// The engine should call it once per digit (typically 8 passes for u32 keys
/// with 4-bit radix), alternating input and output buffers each pass.
///
/// Algorithm: local histogram → prefix sum → scatter.
const SORTING_WGSL: &str = r#"
struct SortUniforms {
    element_count: u32,
    shift: u32,           // bit shift for current digit (e.g. 0, 4, 8, ..., 28)
    radix_mask: u32,      // (1 << bits_per_digit) - 1
    workgroup_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<uniform> uniforms: SortUniforms;
@group(0) @binding(1) var<storage, read> input_keys: array<u32>;
@group(0) @binding(2) var<storage, read> input_values: array<u32>;
@group(0) @binding(3) var<storage, read_write> output_keys: array<u32>;
@group(0) @binding(4) var<storage, read_write> output_values: array<u32>;
@group(0) @binding(5) var<storage, read_write> histogram: array<u32>;

const WORKGROUP_SIZE: u32 = 256u;
const RADIX_SIZE: u32 = 16u; // 4-bit radix

// Local shared memory for per-workgroup histogram.
var<workgroup> local_hist: array<u32, 16>;

@compute @workgroup_size(256, 1, 1)
fn histogram_pass(@builtin(global_invocation_id) gid: vec3<u32>,
                  @builtin(local_invocation_id) lid: vec3<u32>,
                  @builtin(workgroup_id) wid: vec3<u32>) {
    // Zero local histogram.
    if lid.x < RADIX_SIZE {
        local_hist[lid.x] = 0u;
    }
    workgroupBarrier();

    // Each thread tallies its element.
    let idx = gid.x;
    if idx < uniforms.element_count {
        let key = input_keys[idx];
        let digit = (key >> uniforms.shift) & uniforms.radix_mask;
        // Use atomicAdd on local_hist for thread safety within the workgroup.
        // WGSL shared memory atomics require storage class; using a simple
        // serial approach here for maximum compatibility.
        atomicAdd(&local_hist[digit], 1u);
    }
    workgroupBarrier();

    // Write local histogram to global histogram.
    if lid.x < RADIX_SIZE {
        histogram[wid.x * RADIX_SIZE + lid.x] = local_hist[lid.x];
    }
}

// Prefix-sum pass (single-threaded for simplicity; production impl would
// use an exclusive scan compute shader or Blelloch scan).
@compute @workgroup_size(1, 1, 1)
fn prefix_sum_pass() {
    let wg_count = uniforms.workgroup_count;
    // First digit: per-workgroup exclusive prefix sum.
    for (var d = 0u; d < RADIX_SIZE; d++) {
        var running_sum = 0u;
        for (var w = 0u; w < wg_count; w++) {
            let old = histogram[w * RADIX_SIZE + d];
            histogram[w * RADIX_SIZE + d] = running_sum;
            running_sum += old;
        }
    }
}

// Scatter pass: write elements to output buffer at computed positions.
@compute @workgroup_size(256, 1, 1)
fn scatter_pass(@builtin(global_invocation_id) gid: vec3<u32>,
                @builtin(local_invocation_id) lid: vec3<u32>,
                @builtin(workgroup_id) wid: vec3<u32>) {
    var<workgroup> local_hist: array<u32, 16>;

    // Rebuild local histogram for this workgroup.
    if lid.x < RADIX_SIZE {
        local_hist[lid.x] = 0u;
    }
    workgroupBarrier();

    let idx = gid.x;
    var local_offset: u32 = 0u;
    if idx < uniforms.element_count {
        let key = input_keys[idx];
        let digit = (key >> uniforms.shift) & uniforms.radix_mask;
        local_offset = atomicAdd(&local_hist[digit], 1u);
    }
    workgroupBarrier();

    // Scatter.
    if idx < uniforms.element_count {
        let key = input_keys[idx];
        let value = input_values[idx];
        let digit = (key >> uniforms.shift) & uniforms.radix_mask;

        // Global offset = histogram prefix sum + local offset.
        let global_offset = histogram[wid.x * RADIX_SIZE + digit] + local_offset;
        output_keys[global_offset] = key;
        output_values[global_offset] = value;
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorting_shader_defaults() {
        let s = SortingComputeShader::new();
        assert_eq!(s.config.element_count, 1024);
        assert_eq!(s.config.bits_per_digit, 4);
        assert_eq!(s.config.key_bits, 32);
    }

    #[test]
    fn sorting_shader_source_non_empty() {
        let s = SortingComputeShader::new();
        assert!(!s.source().is_empty());
        assert!(s.source().contains("@compute"));
        assert!(s.source().contains("histogram_pass"));
        assert!(s.source().contains("scatter_pass"));
    }

    #[test]
    fn sorting_shader_for_count() {
        let s = SortingComputeShader::for_count(4096);
        assert_eq!(s.config.element_count, 4096);
    }

    #[test]
    fn sorting_shader_with_params() {
        let s = SortingComputeShader::with_params(2048, 2, 16);
        assert_eq!(s.config.element_count, 2048);
        assert_eq!(s.config.bits_per_digit, 2);
        assert_eq!(s.config.key_bits, 16);
    }

    #[test]
    fn sorting_config_digit_count() {
        let cfg = SortingConfig::default(); // 4-bit digits, 32-bit key
        assert_eq!(cfg.digit_count(), 8); // 32 / 4 = 8
    }

    #[test]
    fn sorting_config_radix_size() {
        let cfg = SortingConfig::default();
        assert_eq!(cfg.radix_size(), 16);
    }

    #[test]
    fn sorting_config_histogram_size() {
        let cfg = SortingConfig::default();
        // 16 radix values × ceil(1024 / 256) = 4 workgroups = 64
        assert_eq!(cfg.histogram_size(256), 64);
    }

    #[test]
    fn bind_unbind_do_not_panic() {
        let s = SortingComputeShader::new();
        s.bind();
        s.unbind();
    }

    #[test]
    fn dispatch_count_for_sorting() {
        let s = SortingComputeShader::for_count(1000);
        let dispatch = s.dispatch_config();
        assert_eq!(dispatch.workgroup_size.x, 256);
        assert_eq!(dispatch.element_count, 1000);
        // ceil(1000 / 256) = 4
        assert_eq!(dispatch.dispatch_count(), (4, 1, 1));
    }
}
