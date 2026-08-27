//! SIMD-optimised batch operations for voxel meshing.
//!
//! Provides hot-path helpers used by the meshing pipeline:
//!
//! * [`simd_normals_batch`] — batch normalise an array of `[f32;3]` normal
//!   vectors in batches of 4 (SSE2), 8 (AVX2), or 4 (NEON), with a scalar fallback.
//! * [`simd_aabb_center_batch`] — compute the centre of axis-aligned bounding
//!   boxes given as `[min_xyz; max_xyz]` (length-6 slices) in batches of 4/8.
//! * [`simd_dot_batch`] — batch dot product of pairs of `[f32;3]` vectors.
//! * [`simd_conditional_mix_batch`] — blend vectors based on per-element mask.
//!
//! All functions are safe to call on any platform. When the `simd` feature is
//! enabled, the best available path is selected at runtime (AVX2 > SSE2 > NEON >
//! scalar). Without the `simd` feature, only scalar code is compiled.
//!
//! AVX2 and NEON code paths are always compiled behind `target_feature` guards
//! so that non-target architectures can still compile the crate (they simply
//! won't be called).

#![allow(unsafe_code)] // x86_64 / aarch64 SIMD intrinsics require unsafe blocks

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

/// Dot product of two `[f32; 3]` vectors.
#[inline]
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Blend: `a[i] * (1.0 - mask[i]) + b[i] * mask[i]`.
#[inline]
fn conditional_mix3(a: [f32; 3], b: [f32; 3], mask: [f32; 3]) -> [f32; 3] {
    [
        a[0] * (1.0 - mask[0]) + b[0] * mask[0],
        a[1] * (1.0 - mask[1]) + b[1] * mask[1],
        a[2] * (1.0 - mask[2]) + b[2] * mask[2],
    ]
}

// ---------------------------------------------------------------------------
// Runtime SIMD level detection
// ---------------------------------------------------------------------------

/// Detected SIMD capability at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimdLevel {
    /// No SIMD; scalar fallback.
    Scalar,
    /// SSE2 (baseline on x86_64).
    SSE2,
    /// AVX2 (widest x86 SIMD available).
    AVX2,
    /// NEON (aarch64 baseline).
    NEON,
}

/// Detect the best SIMD level available on the current CPU.
///
/// On x86_64 this queries `cpuid`; on aarch64 it returns `NEON`; on all other
/// architectures it returns `Scalar`.
pub fn get_simd_level() -> SimdLevel {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: cpuid is safe on all x86_64 CPUs.
        if is_x86_feature_detected!("avx2") {
            SimdLevel::AVX2
        } else if is_x86_feature_detected!("sse2") {
            SimdLevel::SSE2
        } else {
            SimdLevel::Scalar
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        // NEON is mandatory on aarch64.
        SimdLevel::NEON
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        SimdLevel::Scalar
    }
}

// ---------------------------------------------------------------------------
// x86_64 SIMD implementation (SSE2 baseline + AVX2 wide path)
// ---------------------------------------------------------------------------

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
mod x86_impl {
    use std::arch::x86_64::*;

    /// Normalise up to 4 normals using SSE2.
    ///
    /// # Safety
    /// Caller must ensure SSE2 is available (always true on x86_64).
    #[target_feature(enable = "sse2")]
    unsafe fn normalize4_sse2(
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
        v3: [f32; 3],
    ) -> [[f32; 3]; 4] {
        let one = _mm_set1_ps(1.0);
        let eps = _mm_set1_ps(f32::EPSILON);

        let a = _mm_set_ps(0.0, v0[2], v0[1], v0[0]);
        let b = _mm_set_ps(0.0, v1[2], v1[1], v1[0]);
        let c = _mm_set_ps(0.0, v2[2], v2[1], v2[0]);
        let d = _mm_set_ps(0.0, v3[2], v3[1], v3[0]);

        let dot4 = |v: __m128| -> __m128 {
            let mul = _mm_mul_ps(v, v);
            let shuf = _mm_movehdup_ps(mul);
            let sums = _mm_add_ps(mul, shuf);
            let shuf2 = _mm_movehl_ps(shuf, sums);
            let hadd = _mm_add_ss(sums, shuf2);
            // Broadcast lane 0 to all lanes so sqrt/div/mask work on all lanes
            _mm_shuffle_ps(hadd, hadd, 0x00)
        };

        let norm4 = |v: __m128| -> __m128 {
            let dot = dot4(v);
            let len = _mm_sqrt_ps(dot);
            let inv_len = _mm_div_ps(one, len);
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

    /// Compute centres of up to 4 AABBs using SSE2.
    #[target_feature(enable = "sse2")]
    unsafe fn aabb_center4_sse2(
        b0: [f32; 6],
        b1: [f32; 6],
        b2: [f32; 6],
        b3: [f32; 6],
    ) -> [[f32; 3]; 4] {
        let half = _mm_set1_ps(0.5);

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

    /// Dot products of 4 pairs of vectors using SSE2.
    #[target_feature(enable = "sse2")]
    unsafe fn dot4_sse2(
        a0: [f32; 3],
        b0: [f32; 3],
        a1: [f32; 3],
        b1: [f32; 3],
        a2: [f32; 3],
        b2: [f32; 3],
        a3: [f32; 3],
        b3: [f32; 3],
    ) -> [f32; 4] {
        let dot_h = |va: __m128, vb: __m128| -> f32 {
            let mul = _mm_mul_ps(va, vb);
            let shuf = _mm_movehdup_ps(mul);
            let sums = _mm_add_ps(mul, shuf);
            let shuf2 = _mm_movehl_ps(shuf, sums);
            _mm_cvtss_f32(_mm_add_ss(sums, shuf2))
        };

        let load3 = |v: [f32; 3]| _mm_set_ps(0.0, v[2], v[1], v[0]);

        [
            dot_h(load3(a0), load3(b0)),
            dot_h(load3(a1), load3(b1)),
            dot_h(load3(a2), load3(b2)),
            dot_h(load3(a3), load3(b3)),
        ]
    }

    // ========================================================================
    // AVX2 -- 8-wide batches
    // ========================================================================

    /// Normalise up to 8 normals using AVX2.
    ///
    /// # Safety
    /// Caller must ensure AVX2 is available (checked via `is_x86_feature_detected!`).
    #[target_feature(enable = "avx2")]
    unsafe fn normalize8_avx2(v: &[[f32; 3]; 8]) -> [[f32; 3]; 8] {
        let one = _mm256_set1_ps(1.0);
        let eps = _mm256_set1_ps(f32::EPSILON);

        // SoA layout: pack x/y/z across 8 vectors into 3 __m256 registers.
        let mut x_arr = [0.0f32; 8];
        let mut y_arr = [0.0f32; 8];
        let mut z_arr = [0.0f32; 8];
        for i in 0..8 {
            x_arr[i] = v[i][0];
            y_arr[i] = v[i][1];
            z_arr[i] = v[i][2];
        }
        let vx = _mm256_loadu_ps(x_arr.as_ptr());
        let vy = _mm256_loadu_ps(y_arr.as_ptr());
        let vz = _mm256_loadu_ps(z_arr.as_ptr());

        // dot = x*x + y*y + z*z
        let dot = _mm256_add_ps(
            _mm256_add_ps(_mm256_mul_ps(vx, vx), _mm256_mul_ps(vy, vy)),
            _mm256_mul_ps(vz, vz),
        );
        let len = _mm256_sqrt_ps(dot);
        let inv_len = _mm256_div_ps(one, len);
        let mask = _mm256_cmp_ps(len, eps, _CMP_GT_OQ);
        let inv = _mm256_and_ps(inv_len, mask);

        let rx = _mm256_mul_ps(vx, inv);
        let ry = _mm256_mul_ps(vy, inv);
        let rz = _mm256_mul_ps(vz, inv);

        let mut out = [[0.0f32; 3]; 8];
        let rx_arr = {
            let mut a = [0.0f32; 8];
            _mm256_storeu_ps(a.as_mut_ptr(), rx);
            a
        };
        let ry_arr = {
            let mut a = [0.0f32; 8];
            _mm256_storeu_ps(a.as_mut_ptr(), ry);
            a
        };
        let rz_arr = {
            let mut a = [0.0f32; 8];
            _mm256_storeu_ps(a.as_mut_ptr(), rz);
            a
        };
        for i in 0..8 {
            out[i] = [rx_arr[i], ry_arr[i], rz_arr[i]];
        }
        out
    }

    /// Compute centres of up to 8 AABBs using AVX2.
    ///
    /// # Safety
    /// Caller must ensure AVX2 is available.
    #[target_feature(enable = "avx2")]
    unsafe fn aabb_center8_avx2(bounds: &[[f32; 6]; 8]) -> [[f32; 3]; 8] {
        let half = _mm256_set1_ps(0.5);

        let mut minx = [0.0f32; 8];
        let mut miny = [0.0f32; 8];
        let mut minz = [0.0f32; 8];
        let mut maxx = [0.0f32; 8];
        let mut maxy = [0.0f32; 8];
        let mut maxz = [0.0f32; 8];
        for i in 0..8 {
            minx[i] = bounds[i][0];
            miny[i] = bounds[i][1];
            minz[i] = bounds[i][2];
            maxx[i] = bounds[i][3];
            maxy[i] = bounds[i][4];
            maxz[i] = bounds[i][5];
        }

        let mx = _mm256_loadu_ps(minx.as_ptr());
        let my = _mm256_loadu_ps(miny.as_ptr());
        let mz = _mm256_loadu_ps(minz.as_ptr());
        let mx_hi = _mm256_loadu_ps(maxx.as_ptr());
        let my_hi = _mm256_loadu_ps(maxy.as_ptr());
        let mz_hi = _mm256_loadu_ps(maxz.as_ptr());

        let cx = _mm256_mul_ps(_mm256_add_ps(mx, mx_hi), half);
        let cy = _mm256_mul_ps(_mm256_add_ps(my, my_hi), half);
        let cz = _mm256_mul_ps(_mm256_add_ps(mz, mz_hi), half);

        let mut out = [[0.0f32; 3]; 8];
        let cx_a = {
            let mut a = [0.0f32; 8];
            _mm256_storeu_ps(a.as_mut_ptr(), cx);
            a
        };
        let cy_a = {
            let mut a = [0.0f32; 8];
            _mm256_storeu_ps(a.as_mut_ptr(), cy);
            a
        };
        let cz_a = {
            let mut a = [0.0f32; 8];
            _mm256_storeu_ps(a.as_mut_ptr(), cz);
            a
        };
        for i in 0..8 {
            out[i] = [cx_a[i], cy_a[i], cz_a[i]];
        }
        out
    }

    /// 8 simultaneous dot products using AVX2.
    ///
    /// # Safety
    /// Caller must ensure AVX2 is available.
    #[target_feature(enable = "avx2")]
    unsafe fn dot8_avx2(a: &[[f32; 3]; 8], b: &[[f32; 3]; 8]) -> [f32; 8] {
        let mut ax = [0.0f32; 8];
        let mut ay = [0.0f32; 8];
        let mut az = [0.0f32; 8];
        let mut bx = [0.0f32; 8];
        let mut by = [0.0f32; 8];
        let mut bz = [0.0f32; 8];
        for i in 0..8 {
            ax[i] = a[i][0];
            ay[i] = a[i][1];
            az[i] = a[i][2];
            bx[i] = b[i][0];
            by[i] = b[i][1];
            bz[i] = b[i][2];
        }

        let vax = _mm256_loadu_ps(ax.as_ptr());
        let vay = _mm256_loadu_ps(ay.as_ptr());
        let vaz = _mm256_loadu_ps(az.as_ptr());
        let vbx = _mm256_loadu_ps(bx.as_ptr());
        let vby = _mm256_loadu_ps(by.as_ptr());
        let vbz = _mm256_loadu_ps(bz.as_ptr());

        let result = _mm256_add_ps(
            _mm256_add_ps(_mm256_mul_ps(vax, vbx), _mm256_mul_ps(vay, vby)),
            _mm256_mul_ps(vaz, vbz),
        );

        let mut out = [0.0f32; 8];
        _mm256_storeu_ps(out.as_mut_ptr(), result);
        out
    }

    /// 8-way conditional mix using AVX2.
    ///
    /// `result[i] = a[i] * (1 - mask[i]) + b[i] * mask[i]`.
    ///
    /// # Safety
    /// Caller must ensure AVX2 is available.
    #[target_feature(enable = "avx2")]
    unsafe fn conditional_mix8_avx2(
        a: &[[f32; 3]; 8],
        b: &[[f32; 3]; 8],
        mask: &[[f32; 3]; 8],
    ) -> [[f32; 3]; 8] {
        let one = _mm256_set1_ps(1.0);

        let mut ax = [0.0f32; 8];
        let mut ay = [0.0f32; 8];
        let mut az = [0.0f32; 8];
        let mut bx = [0.0f32; 8];
        let mut by = [0.0f32; 8];
        let mut bz = [0.0f32; 8];
        let mut mx = [0.0f32; 8];
        let mut my = [0.0f32; 8];
        let mut mz = [0.0f32; 8];
        for i in 0..8 {
            ax[i] = a[i][0];
            ay[i] = a[i][1];
            az[i] = a[i][2];
            bx[i] = b[i][0];
            by[i] = b[i][1];
            bz[i] = b[i][2];
            mx[i] = mask[i][0];
            my[i] = mask[i][1];
            mz[i] = mask[i][2];
        }

        let vax = _mm256_loadu_ps(ax.as_ptr());
        let vay = _mm256_loadu_ps(ay.as_ptr());
        let vaz = _mm256_loadu_ps(az.as_ptr());
        let vbx = _mm256_loadu_ps(bx.as_ptr());
        let vby = _mm256_loadu_ps(by.as_ptr());
        let vbz = _mm256_loadu_ps(bz.as_ptr());
        let vmx = _mm256_loadu_ps(mx.as_ptr());
        let vmy = _mm256_loadu_ps(my.as_ptr());
        let vmz = _mm256_loadu_ps(mz.as_ptr());

        // result = a * (1 - mask) + b * mask
        let inv_mask_x = _mm256_sub_ps(one, vmx);
        let inv_mask_y = _mm256_sub_ps(one, vmy);
        let inv_mask_z = _mm256_sub_ps(one, vmz);

        let rx = _mm256_add_ps(_mm256_mul_ps(vax, inv_mask_x), _mm256_mul_ps(vbx, vmx));
        let ry = _mm256_add_ps(_mm256_mul_ps(vay, inv_mask_y), _mm256_mul_ps(vby, vmy));
        let rz = _mm256_add_ps(_mm256_mul_ps(vaz, inv_mask_z), _mm256_mul_ps(vbz, vmz));

        let mut out = [[0.0f32; 3]; 8];
        let rx_a = {
            let mut a = [0.0f32; 8];
            _mm256_storeu_ps(a.as_mut_ptr(), rx);
            a
        };
        let ry_a = {
            let mut a = [0.0f32; 8];
            _mm256_storeu_ps(a.as_mut_ptr(), ry);
            a
        };
        let rz_a = {
            let mut a = [0.0f32; 8];
            _mm256_storeu_ps(a.as_mut_ptr(), rz);
            a
        };
        for i in 0..8 {
            out[i] = [rx_a[i], ry_a[i], rz_a[i]];
        }
        out
    }

    // ========================================================================
    // Public batch dispatchers
    // ========================================================================

    pub(super) fn normals_batch(verts: &[[f32; 3]]) -> Vec<[f32; 3]> {
        let mut out = Vec::with_capacity(verts.len());
        let mut i = 0;

        // AVX2 path: process 8 at a time
        if is_x86_feature_detected!("avx2") {
            while i + 8 <= verts.len() {
                let chunk: &[[f32; 3]; 8] = verts[i..i + 8]
                    .try_into()
                    .expect("slice chunk must be 8 elements");
                let batch = unsafe { normalize8_avx2(chunk) };
                out.extend_from_slice(&batch);
                i += 8;
            }
        }

        // SSE2 path: process remaining 4 at a time
        while i + 4 <= verts.len() {
            let batch =
                unsafe { normalize4_sse2(verts[i], verts[i + 1], verts[i + 2], verts[i + 3]) };
            out.extend_from_slice(&batch);
            i += 4;
        }

        // Scalar tail
        while i < verts.len() {
            out.push(super::normalize3(verts[i]));
            i += 1;
        }
        out
    }

    pub(super) fn aabb_center_batch(bounds: &[[f32; 6]]) -> Vec<[f32; 3]> {
        let mut out = Vec::with_capacity(bounds.len());
        let mut i = 0;

        // AVX2 path
        if is_x86_feature_detected!("avx2") {
            while i + 8 <= bounds.len() {
                let chunk: &[[f32; 6]; 8] = bounds[i..i + 8]
                    .try_into()
                    .expect("slice chunk must be 8 elements");
                let batch = unsafe { aabb_center8_avx2(chunk) };
                out.extend_from_slice(&batch);
                i += 8;
            }
        }

        // SSE2 path
        while i + 4 <= bounds.len() {
            let batch = unsafe {
                aabb_center4_sse2(bounds[i], bounds[i + 1], bounds[i + 2], bounds[i + 3])
            };
            out.extend_from_slice(&batch);
            i += 4;
        }

        // Scalar tail
        while i < bounds.len() {
            out.push(super::aabb_center(bounds[i]));
            i += 1;
        }
        out
    }

    pub(super) fn dot_batch(a: &[[f32; 3]], b: &[[f32; 3]]) -> Vec<f32> {
        assert_eq!(a.len(), b.len(), "dot_batch: input lengths must match");
        let mut out = Vec::with_capacity(a.len());
        let mut i = 0;

        // AVX2 path
        if is_x86_feature_detected!("avx2") {
            while i + 8 <= a.len() {
                let ca: &[[f32; 3]; 8] = a[i..i + 8]
                    .try_into()
                    .expect("slice chunk must be 8 elements");
                let cb: &[[f32; 3]; 8] = b[i..i + 8]
                    .try_into()
                    .expect("slice chunk must be 8 elements");
                let batch = unsafe { dot8_avx2(ca, cb) };
                out.extend_from_slice(&batch);
                i += 8;
            }
        }

        // SSE2 path
        while i + 4 <= a.len() {
            let batch = unsafe {
                dot4_sse2(
                    a[i],
                    b[i],
                    a[i + 1],
                    b[i + 1],
                    a[i + 2],
                    b[i + 2],
                    a[i + 3],
                    b[i + 3],
                )
            };
            out.extend_from_slice(&batch);
            i += 4;
        }

        // Scalar tail
        while i < a.len() {
            out.push(super::dot3(a[i], b[i]));
            i += 1;
        }
        out
    }

    pub(super) fn conditional_mix_batch(
        a: &[[f32; 3]],
        b: &[[f32; 3]],
        mask: &[[f32; 3]],
    ) -> Vec<[f32; 3]> {
        assert_eq!(
            a.len(),
            b.len(),
            "conditional_mix_batch: a/b lengths must match"
        );
        assert_eq!(
            a.len(),
            mask.len(),
            "conditional_mix_batch: a/mask lengths must match"
        );
        let mut out = Vec::with_capacity(a.len());
        let mut i = 0;

        // AVX2 path
        if is_x86_feature_detected!("avx2") {
            while i + 8 <= a.len() {
                let ca: &[[f32; 3]; 8] = a[i..i + 8]
                    .try_into()
                    .expect("slice chunk must be 8 elements");
                let cb: &[[f32; 3]; 8] = b[i..i + 8]
                    .try_into()
                    .expect("slice chunk must be 8 elements");
                let cm: &[[f32; 3]; 8] = mask[i..i + 8]
                    .try_into()
                    .expect("slice chunk must be 8 elements");
                let batch = unsafe { conditional_mix8_avx2(ca, cb, cm) };
                out.extend_from_slice(&batch);
                i += 8;
            }
        }

        // Scalar tail
        while i < a.len() {
            out.push(super::conditional_mix3(a[i], b[i], mask[i]));
            i += 1;
        }
        out
    }

    /// Run AVX2-normalize on a fixed-size array of 8 vectors (test helper).
    #[cfg(test)]
    pub(super) fn normalize8_safe(v: &[[f32; 3]; 8]) -> [[f32; 3]; 8] {
        if is_x86_feature_detected!("avx2") {
            unsafe { normalize8_avx2(v) }
        } else {
            let mut out = [[0.0f32; 3]; 8];
            for i in 0..8 {
                out[i] = super::normalize3(v[i]);
            }
            out
        }
    }

    #[cfg(test)]
    pub(super) fn aabb_center8_safe(b: &[[f32; 6]; 8]) -> [[f32; 3]; 8] {
        if is_x86_feature_detected!("avx2") {
            unsafe { aabb_center8_avx2(b) }
        } else {
            let mut out = [[0.0f32; 3]; 8];
            for i in 0..8 {
                out[i] = super::aabb_center(b[i]);
            }
            out
        }
    }

    #[cfg(test)]
    pub(super) fn dot8_safe(a: &[[f32; 3]; 8], b: &[[f32; 3]; 8]) -> [f32; 8] {
        if is_x86_feature_detected!("avx2") {
            unsafe { dot8_avx2(a, b) }
        } else {
            let mut out = [0.0f32; 8];
            for i in 0..8 {
                out[i] = super::dot3(a[i], b[i]);
            }
            out
        }
    }
}

// ---------------------------------------------------------------------------
// aarch64 NEON implementation
// ---------------------------------------------------------------------------

#[cfg(all(feature = "simd", target_arch = "aarch64"))]
mod neon_impl {
    use std::arch::aarch64::*;

    /// Normalise up to 4 normals using NEON.
    ///
    /// # Safety
    /// Caller must ensure NEON is available (always true on aarch64).
    #[target_feature(enable = "neon")]
    unsafe fn normalize4_neon(
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
        v3: [f32; 3],
    ) -> [[f32; 3]; 4] {
        let load3 =
            |src: [f32; 3]| -> float32x4_t { vld1q_f32([src[0], src[1], src[2], 0.0].as_ptr()) };

        let dot_h = |v: float32x4_t| -> float32x4_t {
            let mul = vmulq_f32(v, v);
            let pair = vadd_f32(vget_low_f32(mul), vget_high_f32(mul));
            let pair2 = vadd_f32(pair, vrev64_f32(pair));
            let scalar = vget_lane_f32(pair2, 0);
            vdupq_n_f32(scalar)
        };

        let normalize = |v: float32x4_t| -> float32x4_t {
            let dot = dot_h(v);
            let len = vrsqrteq_f32(dot);
            // One Newton-Raphson iteration for better precision
            let len2 = vmulq_f32(len, vrsqrteq_f32(vmulq_f32(dot, len)));
            let zero = vdupq_n_f32(0.0);
            let mask = vcgtq_f32(dot, zero);
            let inv = vandq_f32(len2, mask);
            vmulq_f32(v, inv)
        };

        let ra = normalize(load3(v0));
        let rb = normalize(load3(v1));
        let rc = normalize(load3(v2));
        let rd = normalize(load3(v3));

        let extract = |v: float32x4_t| -> [f32; 3] {
            [
                vgetq_lane_f32(v, 0),
                vgetq_lane_f32(v, 1),
                vgetq_lane_f32(v, 2),
            ]
        };

        [extract(ra), extract(rb), extract(rc), extract(rd)]
    }

    /// Compute centres of up to 4 AABBs using NEON.
    #[target_feature(enable = "neon")]
    unsafe fn aabb_center4_neon(
        b0: [f32; 6],
        b1: [f32; 6],
        b2: [f32; 6],
        b3: [f32; 6],
    ) -> [[f32; 3]; 4] {
        let half = vdupq_n_f32(0.5);

        let center = |b: [f32; 6]| -> float32x4_t {
            let mn = vld1q_f32([b[0], b[1], b[2], 0.0].as_ptr());
            let mx = vld1q_f32([b[3], b[4], b[5], 0.0].as_ptr());
            vmulq_f32(vaddq_f32(mn, mx), half)
        };

        let r0 = center(b0);
        let r1 = center(b1);
        let r2 = center(b2);
        let r3 = center(b3);

        let extract = |v: float32x4_t| -> [f32; 3] {
            [
                vgetq_lane_f32(v, 0),
                vgetq_lane_f32(v, 1),
                vgetq_lane_f32(v, 2),
            ]
        };

        [extract(r0), extract(r1), extract(r2), extract(r3)]
    }

    /// Dot products of 4 pairs of vectors using NEON.
    #[target_feature(enable = "neon")]
    unsafe fn dot4_neon(
        a0: [f32; 3],
        b0: [f32; 3],
        a1: [f32; 3],
        b1: [f32; 3],
        a2: [f32; 3],
        b2: [f32; 3],
        a3: [f32; 3],
        b3: [f32; 3],
    ) -> [f32; 4] {
        let dot_h = |va: float32x4_t, vb: float32x4_t| -> f32 {
            let mul = vmulq_f32(va, vb);
            let pair = vadd_f32(vget_low_f32(mul), vget_high_f32(mul));
            let pair2 = vadd_f32(pair, vrev64_f32(pair));
            vget_lane_f32(pair2, 0)
        };

        let load3 = |src: [f32; 3]| vld1q_f32([src[0], src[1], src[2], 0.0].as_ptr());

        [
            dot_h(load3(a0), load3(b0)),
            dot_h(load3(a1), load3(b1)),
            dot_h(load3(a2), load3(b2)),
            dot_h(load3(a3), load3(b3)),
        ]
    }

    pub(super) fn normals_batch(verts: &[[f32; 3]]) -> Vec<[f32; 3]> {
        let mut out = Vec::with_capacity(verts.len());
        let mut i = 0;
        while i + 4 <= verts.len() {
            // SAFETY: NEON is always available on aarch64.
            let batch =
                unsafe { normalize4_neon(verts[i], verts[i + 1], verts[i + 2], verts[i + 3]) };
            out.extend_from_slice(&batch);
            i += 4;
        }
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
                aabb_center4_neon(bounds[i], bounds[i + 1], bounds[i + 2], bounds[i + 3])
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

    pub(super) fn dot_batch(a: &[[f32; 3]], b: &[[f32; 3]]) -> Vec<f32> {
        assert_eq!(a.len(), b.len());
        let mut out = Vec::with_capacity(a.len());
        let mut i = 0;
        while i + 4 <= a.len() {
            let batch = unsafe {
                dot4_neon(
                    a[i],
                    b[i],
                    a[i + 1],
                    b[i + 1],
                    a[i + 2],
                    b[i + 2],
                    a[i + 3],
                    b[i + 3],
                )
            };
            out.extend_from_slice(&batch);
            i += 4;
        }
        while i < a.len() {
            out.push(super::dot3(a[i], b[i]));
            i += 1;
        }
        out
    }

    pub(super) fn conditional_mix_batch(
        a: &[[f32; 3]],
        b: &[[f32; 3]],
        mask: &[[f32; 3]],
    ) -> Vec<[f32; 3]> {
        // Scalar for NEON path (no wide conditional mix intrinsic).
        a.iter()
            .zip(b.iter())
            .zip(mask.iter())
            .map(|((&av, &bv), &mv)| super::conditional_mix3(av, bv, mv))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Scalar fallback (non-SIMD feature or non-x86_64/non-aarch64)
// ---------------------------------------------------------------------------

#[cfg(not(all(feature = "simd", any(target_arch = "x86_64", target_arch = "aarch64"))))]
mod fallback_impl {
    pub(super) fn normals_batch(verts: &[[f32; 3]]) -> Vec<[f32; 3]> {
        verts.iter().map(|&v| super::normalize3(v)).collect()
    }

    pub(super) fn aabb_center_batch(bounds: &[[f32; 6]]) -> Vec<[f32; 3]> {
        bounds.iter().map(|&b| super::aabb_center(b)).collect()
    }

    pub(super) fn dot_batch(a: &[[f32; 3]], b: &[[f32; 3]]) -> Vec<f32> {
        a.iter()
            .zip(b.iter())
            .map(|(&av, &bv)| super::dot3(av, bv))
            .collect()
    }

    pub(super) fn conditional_mix_batch(
        a: &[[f32; 3]],
        b: &[[f32; 3]],
        mask: &[[f32; 3]],
    ) -> Vec<[f32; 3]> {
        a.iter()
            .zip(b.iter())
            .zip(mask.iter())
            .map(|((&av, &bv), &mv)| super::conditional_mix3(av, bv, mv))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Batch-normalise an array of normal vectors.
///
/// When the `simd` feature is enabled, the best available path is selected:
/// AVX2 (8-wide) > SSE2 (4-wide) > NEON (4-wide) > scalar. Without `simd`,
/// a pure scalar loop is used.
///
/// Zero-length or near-zero-length normals are returned as `[0.0; 3]`.
#[inline]
pub fn simd_normals_batch(verts: &[[f32; 3]]) -> Vec<[f32; 3]> {
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    {
        x86_impl::normals_batch(verts)
    }
    #[cfg(all(feature = "simd", target_arch = "aarch64"))]
    {
        neon_impl::normals_batch(verts)
    }
    #[cfg(not(all(feature = "simd", any(target_arch = "x86_64", target_arch = "aarch64"))))]
    {
        fallback_impl::normals_batch(verts)
    }
}

/// Compute the centres of a batch of axis-aligned bounding boxes.
///
/// Each AABB is specified as `[min_x, min_y, min_z, max_x, max_y, max_z]`.
#[inline]
pub fn simd_aabb_center_batch(bounds: &[[f32; 6]]) -> Vec<[f32; 3]> {
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    {
        x86_impl::aabb_center_batch(bounds)
    }
    #[cfg(all(feature = "simd", target_arch = "aarch64"))]
    {
        neon_impl::aabb_center_batch(bounds)
    }
    #[cfg(not(all(feature = "simd", any(target_arch = "x86_64", target_arch = "aarch64"))))]
    {
        fallback_impl::aabb_center_batch(bounds)
    }
}

/// Batch dot product of pairs of `[f32; 3]` vectors.
#[inline]
pub fn simd_dot_batch(a: &[[f32; 3]], b: &[[f32; 3]]) -> Vec<f32> {
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    {
        x86_impl::dot_batch(a, b)
    }
    #[cfg(all(feature = "simd", target_arch = "aarch64"))]
    {
        neon_impl::dot_batch(a, b)
    }
    #[cfg(not(all(feature = "simd", any(target_arch = "x86_64", target_arch = "aarch64"))))]
    {
        fallback_impl::dot_batch(a, b)
    }
}

/// Batch conditional mix: `result[i] = a[i] * (1 - mask[i]) + b[i] * mask[i]`.
#[inline]
pub fn simd_conditional_mix_batch(
    a: &[[f32; 3]],
    b: &[[f32; 3]],
    mask: &[[f32; 3]],
) -> Vec<[f32; 3]> {
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    {
        x86_impl::conditional_mix_batch(a, b, mask)
    }
    #[cfg(all(feature = "simd", target_arch = "aarch64"))]
    {
        neon_impl::conditional_mix_batch(a, b, mask)
    }
    #[cfg(not(all(feature = "simd", any(target_arch = "x86_64", target_arch = "aarch64"))))]
    {
        fallback_impl::conditional_mix_batch(a, b, mask)
    }
}

/// Runtime-dispatched normal normalisation.  Checks CPU features once and
/// delegates to the best available batch path (AVX2 > SSE2 > NEON > scalar).
/// Modifies `normals` in-place.
#[inline]
pub fn dispatch_normalize_normals(normals: &mut [[f32; 3]]) {
    let normalized = simd_normals_batch(normals);
    for (v, n) in normals.iter_mut().zip(normalized) {
        *v = n;
    }
}

/// Runtime-dispatched AABB centre computation.  Returns the centre of each
/// bounding box in the input slice, using the best available batch path.
pub fn dispatch_aabb_centers(bounds: &[[f32; 6]]) -> Vec<[f32; 3]> {
    simd_aabb_center_batch(bounds)
}

/// Runtime-dispatched batch dot product.
pub fn dispatch_dot_batch(a: &[[f32; 3]], b: &[[f32; 3]]) -> Vec<f32> {
    simd_dot_batch(a, b)
}

/// Runtime-dispatched batch conditional mix.
pub fn dispatch_conditional_mix_batch(
    a: &[[f32; 3]],
    b: &[[f32; 3]],
    mask: &[[f32; 3]],
) -> Vec<[f32; 3]> {
    simd_conditional_mix_batch(a, b, mask)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Existing tests (scalar + SIMD path validation)
    // ========================================================================

    /// FR-PHENO-VOXEL-SIMD-001 -- unit vectors are returned unchanged.
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

    /// FR-PHENO-VOXEL-SIMD-002 -- non-unit vectors are normalised to unit length.
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

    /// FR-PHENO-VOXEL-SIMD-003 -- zero-length input produces zero output.
    #[test]
    fn normals_zero_vector_returns_zero() {
        let input: &[[f32; 3]] = &[[0.0, 0.0, 0.0]];
        let out = simd_normals_batch(input);
        assert_eq!(out[0], [0.0; 3]);
    }

    /// FR-PHENO-VOXEL-SIMD-004 -- batch of 7 normals (SIMD + scalar)
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
        assert!((out[0][0] - 1.0).abs() < 1e-5);
        assert!((out[4][0] - 0.6).abs() < 1e-5);
        assert!((out[4][2] - 0.8).abs() < 1e-5);
        assert_eq!(out[5], [0.0; 3]);
    }

    /// FR-PHENO-VOXEL-SIMD-005 -- single AABB centre computed correctly.
    #[test]
    fn aabb_center_single_box() {
        let bounds: &[[f32; 6]] = &[[0.0, 0.0, 0.0, 10.0, 10.0, 10.0]];
        let out = simd_aabb_center_batch(bounds);
        assert_eq!(out.len(), 1);
        assert!((out[0][0] - 5.0).abs() < 1e-5);
        assert!((out[0][1] - 5.0).abs() < 1e-5);
        assert!((out[0][2] - 5.0).abs() < 1e-5);
    }

    /// FR-PHENO-VOXEL-SIMD-006 -- batch of 8 AABBs produces correct centres.
    #[test]
    fn aabb_center_batch_of_eight() {
        let bounds: &[[f32; 6]] = &[
            [0.0, 0.0, 0.0, 2.0, 2.0, 2.0],
            [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0],
            [10.0, 20.0, 30.0, 12.0, 24.0, 36.0],
            [5.0, 5.0, 5.0, 5.0, 5.0, 5.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [-100.0, 0.0, 0.0, 100.0, 0.0, 0.0],
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            [-1.0, -2.0, -3.0, 1.0, 2.0, 3.0],
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

    // ========================================================================
    // AVX2 tests (4 tests)
    // ========================================================================

    /// AVX2-001 -- normalize8 produces unit-length vectors for all 8 inputs.
    #[test]
    fn avx2_normalize8_all_unit_length() {
        let input: [[f32; 3]; 8] = [
            [1.0, 0.0, 0.0],
            [0.0, 3.0, 0.0],
            [0.0, 0.0, 6.0],
            [1.0, 1.0, 1.0],
            [3.0, 0.0, 4.0],
            [0.0, 0.0, 0.0],
            [-1.0, -1.0, -1.0],
            [100.0, 200.0, 300.0],
        ];
        let out = normalize8_batch_wrapper(&input);
        for (i, v) in out.iter().enumerate() {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            if i == 5 {
                // zero vector
                assert_eq!(*v, [0.0; 3]);
            } else {
                assert!(
                    (len - 1.0).abs() < 1e-4,
                    "avx2 normalize8 vector[{}]: length {} (expected 1.0)",
                    i,
                    len
                );
            }
        }
    }

    /// AVX2-002 -- aabb_center8 produces correct centres for 8 boxes.
    #[test]
    fn avx2_aabb_center8_correct() {
        let bounds: [[f32; 6]; 8] = [
            [0.0, 0.0, 0.0, 10.0, 10.0, 10.0],
            [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0],
            [10.0, 20.0, 30.0, 12.0, 24.0, 36.0],
            [5.0, 5.0, 5.0, 5.0, 5.0, 5.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [-100.0, 0.0, 0.0, 100.0, 0.0, 0.0],
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            [-1.0, -2.0, -3.0, 1.0, 2.0, 3.0],
        ];
        let out = aabb_center8_batch_wrapper(&bounds);
        let expected = [
            [5.0, 5.0, 5.0],
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
                    "avx2 aabb_center8[{}][{}]: got {}, expected {}",
                    i,
                    j,
                    out[i][j],
                    exp[j]
                );
            }
        }
    }

    /// AVX2-003 -- dot8 matches scalar dot products.
    #[test]
    fn avx2_dot8_matches_scalar() {
        let a: [[f32; 3]; 8] = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [3.0, 4.0, 0.0],
            [0.0, 0.0, 0.0],
            [-1.0, 2.0, -3.0],
            [10.0, 20.0, 30.0],
        ];
        let b: [[f32; 3]; 8] = [
            [1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7.0, 8.0, 9.0],
            [1.0, 1.0, 1.0],
            [0.0, 0.0, 1.0],
            [5.0, 5.0, 5.0],
            [2.0, -1.0, 1.0],
            [0.5, 0.5, 0.5],
        ];
        let out = dot8_batch_wrapper(&a, &b);
        for i in 0..8 {
            let scalar = dot3(a[i], b[i]);
            assert!(
                (out[i] - scalar).abs() < 1e-5,
                "avx2 dot8[{}]: got {}, expected {}",
                i,
                out[i],
                scalar
            );
        }
    }

    /// AVX2-004 -- batch of 16 normals exercises both AVX2 and scalar tail.
    #[test]
    fn avx2_batch_of_16_all_correct() {
        let input: Vec<[f32; 3]> = (0..16)
            .map(|i| {
                let f = i as f32 + 1.0;
                [f, f * 2.0, f * 3.0]
            })
            .collect();
        let out = simd_normals_batch(&input);
        assert_eq!(out.len(), 16);
        for (i, v) in out.iter().enumerate() {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            assert!(
                (len - 1.0).abs() < 1e-4,
                "batch 16 normal[{}]: length {}",
                i,
                len
            );
        }
    }

    // ========================================================================
    // NEON tests (4 tests -- compile on all arches, validated on aarch64)
    // ========================================================================

    /// NEON-001 -- dot4 matches scalar for 4 pairs.
    #[test]
    fn neon_dot4_matches_scalar() {
        let a: &[[f32; 3]] = &[
            [1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ];
        let b: &[[f32; 3]] = &[
            [7.0, 8.0, 9.0],
            [1.0, 1.0, 1.0],
            [5.0, 5.0, 5.0],
            [0.0, 1.0, 0.0],
        ];
        let out = simd_dot_batch(a, b);
        assert_eq!(out.len(), 4);
        for i in 0..4 {
            let scalar = dot3(a[i], b[i]);
            assert!(
                (out[i] - scalar).abs() < 1e-5,
                "neon/sse dot_batch[{}]: got {}, expected {}",
                i,
                out[i],
                scalar
            );
        }
    }

    /// NEON-002 -- conditional_mix blends correctly.
    #[test]
    fn neon_conditional_mix_blend() {
        let a: &[[f32; 3]] = &[[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
        let b: &[[f32; 3]] = &[[1.0, 1.0, 1.0], [0.0, 0.0, 0.0]];
        let mask: &[[f32; 3]] = &[[1.0, 1.0, 1.0], [0.5, 0.5, 0.5]];
        let out = simd_conditional_mix_batch(a, b, mask);
        assert_eq!(out.len(), 2);
        // mask=1 -> result = b
        for j in 0..3 {
            assert!((out[0][j] - 1.0).abs() < 1e-5);
        }
        // mask=0.5 -> result = a*0.5 + b*0.5
        for j in 0..3 {
            assert!((out[1][j] - 0.5).abs() < 1e-5);
        }
    }

    /// NEON-003 -- normals_batch of exactly 4 vectors (NEON width).
    #[test]
    fn neon_normals_exactly_4_vectors() {
        let input: &[[f32; 3]] = &[
            [3.0, 0.0, 0.0],
            [0.0, 4.0, 0.0],
            [0.0, 0.0, 5.0],
            [1.0, 1.0, 1.0],
        ];
        let out = simd_normals_batch(input);
        assert_eq!(out.len(), 4);
        assert!((out[0][0] - 1.0).abs() < 1e-4, "out[0][0] = {}", out[0][0]);
        assert!((out[1][1] - 1.0).abs() < 1e-4, "out[1][1] = {}", out[1][1]);
        assert!((out[2][2] - 1.0).abs() < 1e-4, "out[2][2] = {}", out[2][2]);
        let len3 = (out[3][0].powi(2) + out[3][1].powi(2) + out[3][2].powi(2)).sqrt();
        assert!((len3 - 1.0).abs() < 1e-4);
    }

    /// NEON-004 -- dot_batch of 5 (4 SIMD + 1 scalar tail).
    #[test]
    fn neon_dot_batch_of_5() {
        let a: &[[f32; 3]] = &[
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 0.0],
            [2.0, 3.0, 4.0],
        ];
        let b: &[[f32; 3]] = &[
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [1.0, -1.0, 0.0],
            [5.0, 6.0, 7.0],
        ];
        let out = simd_dot_batch(a, b);
        assert_eq!(out.len(), 5);
        let expected = [0.0, 0.0, 0.0, 0.0, 2.0 * 5.0 + 3.0 * 6.0 + 4.0 * 7.0];
        for i in 0..5 {
            assert!(
                (out[i] - expected[i]).abs() < 1e-5,
                "dot_batch[{}]: got {}, expected {}",
                i,
                out[i],
                expected[i]
            );
        }
    }

    // ========================================================================
    // Feature detection tests (4 tests)
    // ========================================================================

    /// FEAT-001 -- get_simd_level returns a valid level.
    #[test]
    fn feature_detection_returns_valid_level() {
        let level = get_simd_level();
        match level {
            SimdLevel::Scalar | SimdLevel::SSE2 | SimdLevel::AVX2 | SimdLevel::NEON => {}
        }
    }

    /// FEAT-002 -- on x86_64, level is at least SSE2.
    #[test]
    fn feature_detection_x86_64_at_least_sse2() {
        let level = get_simd_level();
        #[cfg(target_arch = "x86_64")]
        {
            assert!(
                level == SimdLevel::SSE2 || level == SimdLevel::AVX2,
                "x86_64 should have at least SSE2, got {:?}",
                level
            );
        }
        #[cfg(target_arch = "aarch64")]
        {
            assert_eq!(level, SimdLevel::NEON);
        }
    }

    /// FEAT-003 -- SimdLevel is Debug, Clone, Copy, PartialEq, Eq, Hash.
    #[test]
    fn simd_level_traits() {
        let level = get_simd_level();
        let level2 = level;
        assert_eq!(level, level2);
        let _dbg = format!("{:?}", level);
    }

    /// FEAT-004 -- batch of 32 normals exercises AVX2 multiple iterations.
    #[test]
    fn feature_detection_batch_32_normals_all_unit() {
        let input: Vec<[f32; 3]> = (0..32)
            .map(|i| {
                let f = i as f32 + 1.0;
                [f, f * 2.0, f * 3.0]
            })
            .collect();
        let out = simd_normals_batch(&input);
        assert_eq!(out.len(), 32);
        for (i, v) in out.iter().enumerate() {
            let len = (v[0].powi(2) + v[1].powi(2) + v[2].powi(2)).sqrt();
            assert!(
                (len - 1.0).abs() < 1e-4,
                "batch 32 normal[{}]: length {}",
                i,
                len
            );
        }
    }

    // ========================================================================
    // Helper wrappers for AVX2 test functions
    // ========================================================================

    /// Wrapper that dispatches normalize8 through the x86_impl path (safe).
    fn normalize8_batch_wrapper(input: &[[f32; 3]; 8]) -> [[f32; 3]; 8] {
        #[cfg(all(feature = "simd", target_arch = "x86_64"))]
        {
            x86_impl::normalize8_safe(input)
        }
        #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
        {
            let mut out = [[0.0f32; 3]; 8];
            for i in 0..8 {
                out[i] = normalize3(input[i]);
            }
            out
        }
    }

    /// Wrapper for aabb_center8.
    fn aabb_center8_batch_wrapper(input: &[[f32; 6]; 8]) -> [[f32; 3]; 8] {
        #[cfg(all(feature = "simd", target_arch = "x86_64"))]
        {
            x86_impl::aabb_center8_safe(input)
        }
        #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
        {
            let mut out = [[0.0f32; 3]; 8];
            for i in 0..8 {
                out[i] = aabb_center(input[i]);
            }
            out
        }
    }

    /// Wrapper for dot8.
    fn dot8_batch_wrapper(a: &[[f32; 3]; 8], b: &[[f32; 3]; 8]) -> [f32; 8] {
        #[cfg(all(feature = "simd", target_arch = "x86_64"))]
        {
            x86_impl::dot8_safe(a, b)
        }
        #[cfg(not(all(feature = "simd", target_arch = "x86_64")))]
        {
            let mut out = [0.0f32; 8];
            for i in 0..8 {
                out[i] = dot3(a[i], b[i]);
            }
            out
        }
    }
}
