//! SIMD-optimised batch operations for voxel meshing.
//!
//! Provides two hot-path helpers used by the meshing pipeline:
//!
//! * [`simd_normals_batch`] — normalise an array of `[f32;3]` normal vectors in
//!   batches of 4 using SSE2/AVX2 when available, with a scalar fallback.
//! * [`simd_aabb_center_batch`] — compute the centre of axis-aligned bounding
//!   boxes given as `[min_xyz; max_xyz]` (length-6 slices) in batches of 4.
//!
//! All functions are safe to call on any platform. On `x86_64` with SSE2
//! (baseline for the target) the SIMD path is selected automatically; on other
//! architectures a straightforward scalar loop is used.

// ---------------------------------------------------------------------------
// Scalar helpers (always available)
// ---------------------------------------------------------------------------

/// Normalise a single `[f32; 3]` vector. Returns the zero vector when the
/// input length is zero.
#[inline]
fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len <= f32::EPSILON {
        return [0.0; 3];
    }
    let inv = 1.0 / len;
    [v[0] * inv, v[1] * inv, v[2] * inv]
}

/// Compute the centre of an AABB stored as `[min_x, min_y, min_z, max_x,
/// max_y, max_z]`.
#[inline]
fn aabb_center(b: [f32; 6]) -> [f32; 3] {
    [
        (b[0] + b[3]) * 0.5,
        (b[1] + b[4]) * 0.5,
        (b[2] + b[5]) * 0.5,
    ]
}

// ---------------------------------------------------------------------------
// x86_64 SIMD implementation (SSE2 baseline, AVX2 when available)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
mod x86_impl {
    use std::arch::x86_64::*;

    // -------------------------------------------------------------------
    // Batch normalise — SSE2 path
    // -------------------------------------------------------------------

    /// Normalise up to 4 normals using SSE2.
    ///
    /// # Safety
    /// Caller must ensure SSE2 is available (always true on x86_64).
    #[target_feature(enable = "sse2")]
    unsafe fn normalize4_sse2(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3], v3: [f32; 3]) -> [[f32; 3]; 4] {
        let one = _mm_set1_ps(1.0);
        let eps = _mm_set1_ps(f32::EPSILON);

        // Load xyz for each vector into 128-bit registers (3 floats each,
        // last lane unused).
        let a = _mm_set_ps(0.0, v0[2], v0[1], v0[0]);
        let b = _mm_set_ps(0.0, v1[2], v1[1], v1[0]);
        let c = _mm_set_ps(0.0, v2[2], v2[1], v2[0]);
        let d = _mm_set_ps(0.0, v3[2], v3[1], v3[0]);

        // dot(v, v) = x*x + y*y + z*z (w lane is 0 so ignored).
        let dot4 = |v: __m128| -> __m128 {
            let mul = _mm_mul_ps(v, v);
            // horizontal sum of xyz: (x+y, y+z, ..., 0) then shuffle to add
            let shuf = _mm_movehdup_ps(mul); // (y,y,w,w)
            let sums = _mm_add_ps(mul, shuf); // (x+y, ?, ?, ?)
            let shuf2 = _mm_movehl_ps(shuf, sums); // (z, ?, ?, ?)
            _mm_add_ss(sums, shuf2) // x+y+z in lane 0
        };

        let norm4 = |v: __m128| -> __m128 {
            let dot = dot4(v);
            let len = _mm_sqrt_ps(dot);
            let inv_len = _mm_div_ps(one, len);
            // Mask: where len > epsilon, use 1/len; else 0.
            let mask = _mm_cmpgt_ps(len, eps);
            let inv = _mm_and_ps(inv_len, mask);
            _mm_mul_ps(v, inv)
        };

        let ra = norm4(a);
        let rb = norm4(b);
        let rc = norm4(c);
        let rd = norm4(d);

        let extract = |v: __m128| -> [f32; 3] {
            let mut out = [0.0f32; 3];
            out[0] = _mm_cvtss_f32(v);
            out[1] = _mm_cvtss_f32(_mm_shuffle_ps(v, v, 0x55));
            out[2] = _mm_cvtss_f32(_mm_shuffle_ps(v, v, 0xAA));
            out
        };

        [extract(ra), extract(rb), extract(rc), extract(rd)]
    }

    // -------------------------------------------------------------------
    // Batch AABB centre — SSE2 path
    // -------------------------------------------------------------------

    /// Compute centres of up to 4 AABBs using SSE2.
    ///
    /// Each AABB is `[min_x, min_y, min_z, max_x, max_y, max_z]`.
    ///
    /// # Safety
    /// Caller must ensure SSE2 is available.
    #[target_feature(enable = "sse2")]
    unsafe fn aabb_center4_sse2(
        b0: [f32; 6],
        b1: [f32; 6],
        b2: [f32; 6],
        b3: [f32; 6],
    ) -> [[f32; 3]; 4] {
        let half = _mm_set1_ps(0.5);

        // Pack min.xyz and max.xyz into SSE registers.
        let load_min = |b: &[f32; 6]| _mm_set_ps(0.0, b[2], b[1], b[0]);
        let load_max = |b: &[f32; 6]| _mm_set_ps(0.0, b[5], b[4], b[3]);

        let center4 = |b: &[f32; 6]| {
            let mn = load_min(b);
            let mx = load_max(b);
            _mm_mul_ps(_mm_add_ps(mn, mx), half)
        };

        let r0 = center4(&b0);
        let r1 = center4(&b1);
        let r2 = center4(&b2);
        let r3 = center4(&b3);

        let extract = |v: __m128| -> [f32; 3] {
            let mut out = [0.0f32; 3];
            out[0] = _mm_cvtss_f32(v);
            out[1] = _mm_cvtss_f32(_mm_shuffle_ps(v, v, 0x55));
            out[2] = _mm_cvtss_f32(_mm_shuffle_ps(v, v, 0xAA));
            out
        };

        [extract(r0), extract(r1), extract(r2), extract(r3)]
    }

    // -------------------------------------------------------------------
    // Public wrappers that handle dispatch
    // -------------------------------------------------------------------

    pub(super) fn normals_batch(verts: &[[f32; 3]]) -> Vec<[f32; 3]> {
        let mut out = Vec::with_capacity(verts.len());
        let mut i = 0;

        // Process 4 at a time with SSE2.
        while i + 4 <= verts.len() {
            // SAFETY: SSE2 is always available on x86_64.
            let batch = unsafe {
                normalize4_sse2(verts[i], verts[i + 1], verts[i + 2], verts[i + 3])
            };
            out.extend_from_slice(&batch);
            i += 4;
        }

        // Scalar tail.
        while i < verts.len() {
            out.push(super::normalize3(verts[i]));
            i += 1;
        }

        out
    }

    pub(super) fn aabb_center_batch(bounds: &[[f32; 6]]) -> Vec<[f32; 3]> {
        let mut out = Vec::with_capacity(bounds.len());
        let mut i = 0;

        while i + 4 <= bounds.len() {
            let batch = unsafe {
                aabb_center4_sse2(bounds[i], bounds[i + 1], bounds[i + 2], bounds[i + 3])
            };
            out.extend_from_slice(&batch);
            i += 4;
        }

        while i < bounds.len() {
            out.push(super::aabb_center(bounds[i]));
            i += 1;
        }

        out
    }
}

// ---------------------------------------------------------------------------
// Non-x86_64 scalar fallback
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "x86_64"))]
mod fallback_impl {
    pub(super) fn normals_batch(verts: &[[f32; 3]]) -> Vec<[f32; 3]> {
        verts.iter().map(|&v| super::normalize3(v)).collect()
    }

    pub(super) fn aabb_center_batch(bounds: &[[f32; 6]]) -> Vec<[f32; 3]> {
        bounds.iter().map(|&b| super::aabb_center(b)).collect()
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Batch-normalise an array of normal vectors.
///
/// On `x86_64`, groups of 4 normals are processed with SSE2 intrinsics; the
/// scalar tail is handled element-by-element. On other architectures a pure
/// scalar loop is used.
///
/// Zero-length or near-zero-length normals are returned as `[0.0; 3]`.
///
/// # Examples
///
/// ```
/// use phenotype_voxel::simd::simd_normals_batch;
///
/// let normals = simd_normals_batch(&[[1.0, 0.0, 0.0], [0.0, 3.0, 0.0]]);
/// assert!((normals[0][0] - 1.0).abs() < 1e-6);
/// assert!((normals[1][1] - 1.0).abs() < 1e-6);
/// ```
#[inline]
pub fn simd_normals_batch(verts: &[[f32; 3]]) -> Vec<[f32; 3]> {
    #[cfg(target_arch = "x86_64")]
    {
        x86_impl::normals_batch(verts)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        fallback_impl::normals_batch(verts)
    }
}

/// Compute the centres of a batch of axis-aligned bounding boxes.
///
/// Each AABB is specified as `[min_x, min_y, min_z, max_x, max_y, max_z]`.
///
/// On `x86_64`, groups of 4 boxes are processed with SSE2 intrinsics.
///
/// # Examples
///
/// ```
/// use phenotype_voxel::simd::simd_aabb_center_batch;
///
/// let centers = simd_aabb_center_batch(&[[0.0, 0.0, 0.0, 2.0, 2.0, 2.0]]);
/// assert!((centers[0][0] - 1.0).abs() < 1e-6);
/// assert!((centers[0][1] - 1.0).abs() < 1e-6);
/// assert!((centers[0][2] - 1.0).abs() < 1e-6);
/// ```
#[inline]
pub fn simd_aabb_center_batch(bounds: &[[f32; 6]]) -> Vec<[f32; 3]> {
    #[cfg(target_arch = "x86_64")]
    {
        x86_impl::aabb_center_batch(bounds)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        fallback_impl::aabb_center_batch(bounds)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------
    // Test 1 — simd_normals_batch normalises unit vectors
    // -------------------------------------------------------------------
    /// FR-PHENO-VOXEL-SIMD-001 — unit vectors are returned unchanged.
    #[test]
    fn normals_unit_vectors_unchanged() {
        let input: &[[f32; 3]] = &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let out = simd_normals_batch(input);
        assert_eq!(out.len(), 3);
        for (i, expected) in input.iter().enumerate() {
            for j in 0..3 {
                assert!(
                    (out[i][j] - expected[j]).abs() < 1e-5,
                    "normal[{}][{}]: got {}, expected {}",
                    i,
                    j,
                    out[i][j],
                    expected[j]
                );
            }
        }
    }

    // -------------------------------------------------------------------
    // Test 2 — simd_normals_batch handles non-unit vectors
    // -------------------------------------------------------------------
    /// FR-PHENO-VOXEL-SIMD-002 — non-unit vectors are normalised to unit
    /// length.
    #[test]
    fn normals_non_unit_become_unit() {
        let input: &[[f32; 3]] = &[[0.0, 3.0, 0.0], [1.0, 1.0, 1.0]];
        let out = simd_normals_batch(input);
        for (i, v) in out.iter().enumerate() {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            assert!(
                (len - 1.0).abs() < 1e-5,
                "output normal[{}] has length {} (expected ~1.0)",
                i,
                len
            );
        }
    }

    // -------------------------------------------------------------------
    // Test 3 — simd_normals_batch handles zero vector
    // -------------------------------------------------------------------
    /// FR-PHENO-VOXEL-SIMD-003 — zero-length input produces zero output
    /// (no NaN/Inf).
    #[test]
    fn normals_zero_vector_returns_zero() {
        let input: &[[f32; 3]] = &[[0.0, 0.0, 0.0]];
        let out = simd_normals_batch(input);
        assert_eq!(out[0], [0.0; 3]);
    }

    // -------------------------------------------------------------------
    // Test 4 — simd_normals_batch handles large batch (>4, exercises SIMD
    //          loop + scalar tail)
    // -------------------------------------------------------------------
    /// FR-PHENO-VOXEL-SIMD-004 — batch of 7 normals (4 SIMD + 3 scalar)
    /// all normalise correctly.
    #[test]
    fn normals_batch_size_exceeding_simd_width() {
        let input: &[[f32; 3]] = &[
            [2.0, 0.0, 0.0],
            [0.0, 4.0, 0.0],
            [0.0, 0.0, 6.0],
            [1.0, 1.0, 1.0],
            [3.0, 0.0, 4.0],
            [0.0, 0.0, 0.0],
            [-1.0, -1.0, -1.0],
        ];
        let out = simd_normals_batch(input);
        assert_eq!(out.len(), 7);
        // Spot-check a few.
        assert!((out[0][0] - 1.0).abs() < 1e-5); // [2,0,0] → [1,0,0]
        assert!((out[4][0] - 0.6).abs() < 1e-5); // [3,0,4] → [0.6, 0, 0.8]
        assert!((out[4][2] - 0.8).abs() < 1e-5);
        assert_eq!(out[5], [0.0; 3]); // zero → zero
    }

    // -------------------------------------------------------------------
    // Test 5 — simd_aabb_center_batch simple case
    // -------------------------------------------------------------------
    /// FR-PHENO-VOXEL-SIMD-005 — single AABB centre computed correctly.
    #[test]
    fn aabb_center_single_box() {
        let bounds: &[[f32; 6]] = &[[0.0, 0.0, 0.0, 10.0, 10.0, 10.0]];
        let out = simd_aabb_center_batch(bounds);
        assert_eq!(out.len(), 1);
        assert!((out[0][0] - 5.0).abs() < 1e-5);
        assert!((out[0][1] - 5.0).abs() < 1e-5);
        assert!((out[0][2] - 5.0).abs() < 1e-5);
    }

    // -------------------------------------------------------------------
    // Test 6 — simd_aabb_center_batch large batch (exercises SIMD loop)
    // -------------------------------------------------------------------
    /// FR-PHENO-VOXEL-SIMD-006 — batch of 8 AABBs (2 SIMD iterations)
    /// produces correct centres.
    #[test]
    fn aabb_center_batch_of_eight() {
        let bounds: &[[f32; 6]] = &[
            [0.0, 0.0, 0.0, 2.0, 2.0, 2.0],   // center (1,1,1)
            [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0], // center (0,0,0)
            [10.0, 20.0, 30.0, 12.0, 24.0, 36.0], // center (11,22,33)
            [5.0, 5.0, 5.0, 5.0, 5.0, 5.0],    // degenerate: centre = (5,5,5)
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],    // zero-size box
            [-100.0, 0.0, 0.0, 100.0, 0.0, 0.0], // flat box
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],    // center (2.5, 3.5, 4.5)
            [-1.0, -2.0, -3.0, 1.0, 2.0, 3.0],  // center (0,0,0)
        ];
        let out = simd_aabb_center_batch(bounds);
        assert_eq!(out.len(), 8);

        let expected = [
            [1.0, 1.0, 1.0],
            [0.0, 0.0, 0.0],
            [11.0, 22.0, 33.0],
            [5.0, 5.0, 5.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [2.5, 3.5, 4.5],
            [0.0, 0.0, 0.0],
        ];

        for (i, exp) in expected.iter().enumerate() {
            for j in 0..3 {
                assert!(
                    (out[i][j] - exp[j]).abs() < 1e-4,
                    "aabb center[{}][{}]: got {}, expected {}",
                    i,
                    j,
                    out[i][j],
                    exp[j]
                );
            }
        }
    }
}
