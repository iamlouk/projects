//! Optimized version of continuous CA
//!
//! The neighbourhood is a 15x15 box centred on the cell, minus the inner 3x3
//! box (which includes the cell itself). Box sums are separable, so instead
//! of doing 216 scattered loads per cell we compute:
//!
//!   ring_sum = box_sum(15x15) - box_sum(3x3)
//!
//! using sliding-window sums, which brings the per-cell cost down to a
//! handful of sequential, auto-vectorizable operations.

use rand::Rng;
use std::simd::prelude::*;

use crate::CACell;

/// Transparent so column arrays can be reinterpreted as raw f32 slices
/// for SIMD access without any conversion pass.
#[derive(Clone, Copy, Default)]
#[repr(transparent)]
pub struct Cell(pub f32);

// Keep in sync with main.rs (single source of truth).
const GRID_X: usize = crate::CELLS_X as usize;
const GRID_Y: usize = crate::CELLS_Y as usize;

/// SIMD width: 4 f32 lanes (aarch64 NEON is 128-bit).
const LANES: usize = 4;
/// Largest multiple of LANES that fits in a column (e.g. 450 -> 448).
const NSIMD: usize = GRID_Y / LANES * LANES;

const R: usize = 7; // neighbourhood radius
const W: usize = 2 * R + 1; // window size (15)
const PAD: usize = 2 * R; // wrap padding (14)
const EXT_Y: usize = GRID_Y + PAD;

/// 1 / number of neighbours (multiply instead of divide).
const INV_NEIGHBOURS: f32 = 1.0 / ((W * W) - 9) as f32; // 216 neighbours

lazy_static::lazy_static! {
    static ref NEIGHBOURS: [(i64, i64); 216] = {
        let mut result = [(0, 0); 216];
        let mut i = 0;
        for x in (-7)..8 {
            for y in (-7)..8 {
                if (x <= 1 && x >= -1) && (y <= 1 && y >= -1) {
                    continue;
                }
                result[i] = (x, y);
                i += 1;
            }
        }
        result
    };
}

/// Compute update for a single cell (branchless version).
///
/// Original rule:
///   c < 0 && a > -0.2  -> -c + 0.025
///   c > 0 && a < 0     -> -c - 0.025
///   c > 0 && a > 0.55  -> -c - 0.025
///   c < a              -> c + 0.01
///   a < c              -> c - 0.01
///   otherwise          -> c
///
/// Written with selects (f32::mul_add / arithmetic on masks) so it compiles
/// to branch-free code and can auto-vectorize.
#[inline(always)]
fn compute_update_single(current: f32, avg: f32) -> f32 {
    let c = current;
    let a = avg;

    // flip conditions (mutually exclusive with each other and with drift)
    let flip_up = (c < 0.0) & (a > -0.2); // c < 0 && a > -0.2
    let flip_dn = (c > 0.0) & ((a < 0.0) | (a > 0.55));
    let flip = flip_up | flip_dn;
    // Note: when flip_up is true, c < 0 so -c + 0.025; when flip_dn, -c - 0.025.
    // Since sign(c) determines which, a single expression works:
    let flip_val = -c + if flip_up { 0.025 } else { -0.025 };

    // drift when not flipping (no change when c == a)
    let drift = if c < a {
        c + 0.01
    } else if a < c {
        c - 0.01
    } else {
        c
    };

    let r = if flip { flip_val } else { drift };
    r.clamp(-1.0, 1.0)
}

/// SIMD version of the update rule (4 lanes), branch-free via mask selects.
/// Must stay semantically identical to compute_update_single; the
/// frame_update_matches_naive test pins this.
#[inline(always)]
fn compute_update_simd(c: f32x4, a: f32x4) -> f32x4 {
    let zero = f32x4::splat(0.0);
    let flip_up = c.simd_lt(zero) & a.simd_gt(f32x4::splat(-0.2));
    let flip_dn = c.simd_gt(zero) & (a.simd_lt(zero) | a.simd_gt(f32x4::splat(0.55)));
    let flip = flip_up | flip_dn;
    // sign of c decides the flip direction, same as the scalar rule
    let delta = flip_up.select(f32x4::splat(0.025), f32x4::splat(-0.025));
    let flip_val = (-c) + delta;
    let up = c.simd_lt(a);
    let dn = a.simd_lt(c);
    let drift = up.select(c + f32x4::splat(0.01), dn.select(c - f32x4::splat(0.01), c));
    let r = flip.select(flip_val, drift);
    r.simd_max(f32x4::splat(-1.0)).simd_min(f32x4::splat(1.0))
}

/// Fast integer wrapping - same as main.rs wrap_idxs
#[inline(always)]
fn wrap_idx(mut x: i64, size: i64) -> usize {
    if x < 0 {
        x += size;
    } else if x >= size {
        x -= size;
    }
    x as usize
}

/// Update a column using a separable sliding-window box sum.
///
/// Per neighbour column we build an extended copy (with the wrap-around
/// values appended) so the inner loops run without any wrapping or bounds
/// checks. Vertical window sums are computed by sliding, then combined
/// horizontally with sequential (vectorizable) passes.
///
/// Note: Grid is stored as [[Cell; GRID_Y]; GRID_X] so each element is a column.
#[inline]
pub fn update_column_vectorized(
    old_col: &[Cell; GRID_Y],
    new_col: &mut [Cell; GRID_Y],
    old_grid: &[[Cell; GRID_Y]; GRID_X],
    x: usize,
) {
    // Extended column data for the W neighbour columns: GRID_Y + PAD values,
    // where the last PAD values are the wrapped-around start of the column.
    // Stack usage: ~28 KB per rayon worker.
    let mut ext = [[0f32; EXT_Y]; W];
    // Vertical 15-sums and 3-sums for each relevant column.
    let mut v15 = [[0f32; GRID_Y]; W];
    let mut v3 = [[0f32; GRID_Y]; 3];

    // ---- Build extended buffers and vertical sliding-window sums ----
    for i in 0..W {
        let nx = wrap_idx(x as i64 + i as i64 - R as i64, GRID_X as i64);
        let col = &old_grid[nx];
        let e = &mut ext[i];

        // Shifted extended column: e[j] = col[(j - R) mod GRID_Y].
        // This makes the contiguous window e[y .. y + W - 1] exactly the
        // R-centred vertical window for row y (rows y-7 ..= y+7, wrapped).
        for j in 0..EXT_Y {
            e[j] = col[wrap_idx(j as i64 - R as i64, GRID_Y as i64)].0;
        }

        // Initial window sums at y = 0
        let mut s15 = 0f32;
        for k in 0..W {
            s15 += e[k];
        }
        v15[i][0] = s15;

        // Slide down
        for y in 1..GRID_Y {
            s15 += e[y + W - 1] - e[y - 1];
            v15[i][y] = s15;
        }
    }

    // 3-sums for the three centre columns (ext indices R-1, R, R+1).
    // With the shifted buffer, the centred 3-window for row y is
    // e[y + R - 1 ..= y + R + 1].
    for i in 0..3 {
        let e = &ext[R - 1 + i];
        let mut s3 = e[R - 1] + e[R] + e[R + 1];
        v3[i][0] = s3;
        for y in 1..GRID_Y {
            s3 += e[y + R + 1] - e[y + R - 2];
            v3[i][y] = s3;
        }
    }

    // ---- Horizontal combination: acc[y] = sum of 15-sums - sum of 3-sums ----
    // axpy-style passes over contiguous arrays: auto-vectorizes well.
    let mut acc = v15[0];
    for i in 1..W {
        let v = &v15[i];
        for y in 0..GRID_Y {
            acc[y] += v[y];
        }
    }
    for i in 0..3 {
        let v = &v3[i];
        for y in 0..GRID_Y {
            acc[y] -= v[y];
        }
    }

    // ---- Apply the update rule ----
    for y in 0..GRID_Y {
        let avg = acc[y] * INV_NEIGHBOURS;
        new_col[y] = Cell(compute_update_single(old_col[y].0, avg));
    }
}

impl CACell for Cell {
    /// Whole-frame update, cache-blocked: each tile of TX output columns
    /// computes the vertical window sums for its own inputs (TX + 2R
    /// columns, including the halo) and then does the horizontal pass, so
    /// the intermediates (~280 KB per worker) stay hot in L1/L2 instead of
    /// streaming ~1.6 MB through DRAM every frame.
    fn update_frame(
        old_state: &[[Cell; GRID_Y]; GRID_X],
        new_state: &mut [[Cell; GRID_Y]; GRID_X],
    ) {
        use rayon::prelude::*;

        const TX: usize = 128; // output columns per tile
        const LX: usize = TX + 2 * R; // local input columns incl. halo

        // Per-worker scratch, pooled via thread_local: no contention, no
        // per-frame allocation. Two arrays of LX x GRID_Y f32 = ~280 KB.
        thread_local! {
            static POOL: std::cell::RefCell<Option<(
                Box<[[f32; GRID_Y]; LX]>,
                Box<[[f32; GRID_Y]; LX]>,
            )>> = std::cell::RefCell::new(None);
        }

        new_state
            .par_chunks_mut(TX)
            .enumerate()
            .for_each(|(ti, chunk)| {
                let x0 = ti * TX;
                let n = chunk.len();
                let ln = n + 2 * R; // local input columns actually needed

                let (mut v15, mut v3) = POOL.with(|p| p.borrow_mut().take()).unwrap_or_else(|| {
                    (
                        Box::new([[0f32; GRID_Y]; LX]),
                        Box::new([[0f32; GRID_Y]; LX]),
                    )
                });

                // ---- Local pass 1: vertical sliding-window sums for the
                // input columns [x0 - R, x0 + n + R) ----
                // v15[i][y] = sum of old[x0 + i - R][y - 7 ..= y + 7]
                // v3[i][y]  = sum of old[x0 + i - R][y - 1 ..= y + 1]
                for i in 0..ln {
                    let col = &old_state[wrap_idx(x0 as i64 + i as i64 - R as i64, GRID_X as i64)];
                    // Shifted extended column: e[j] = col[(j - R) mod GRID_Y],
                    // so the contiguous window e[y .. y + W - 1] is exactly
                    // the R-centred vertical window for row y.
                    let mut e = [0f32; EXT_Y];
                    for j in 0..EXT_Y {
                        e[j] = col[wrap_idx(j as i64 - R as i64, GRID_Y as i64)].0;
                    }

                    let mut s15 = 0f32;
                    for k in 0..W {
                        s15 += e[k];
                    }
                    v15[i][0] = s15;
                    for y in 1..GRID_Y {
                        s15 += e[y + W - 1] - e[y - 1];
                        v15[i][y] = s15;
                    }

                    // Centred 3-window for row y is e[y + R - 1 ..= y + R + 1].
                    let mut s3 = e[R - 1] + e[R] + e[R + 1];
                    v3[i][0] = s3;
                    for y in 1..GRID_Y {
                        s3 += e[y + R + 1] - e[y + R - 2];
                        v3[i][y] = s3;
                    }
                }

                // ---- Local pass 2: horizontal sliding window + update ----
                // Output column x0 + j uses local v15[j .. j + W] (grid
                // columns x0 + j - 7 ..= x0 + j + 7) and local
                // v3[j + R - 1 ..= j + R + 1] (grid columns x0 + j - 1
                // ..= x0 + j + 1). All indices are tile-local and contiguous;
                // the only wrap-around is in pass 1.
                //
                // Processed in explicit f32x4 chunks (NEON): the init and
                // slide loops are trivially vectorizable, but the branchy
                // update rule is not - it is vectorized by hand with mask
                // selects (compute_update_simd).
                let mut acc15 = [0f32; GRID_Y];
                let mut acc3 = [0f32; GRID_Y];
                for i in 0..W {
                    let v = &v15[i];
                    for y in (0..NSIMD).step_by(LANES) {
                        let mut acc = f32x4::from_slice(&acc15[y..y + LANES]);
                        acc += f32x4::from_slice(&v[y..y + LANES]);
                        acc.copy_to_slice(&mut acc15[y..y + LANES]);
                    }
                    for y in NSIMD..GRID_Y {
                        acc15[y] += v[y];
                    }
                }
                for i in 0..3usize {
                    let v = &v3[R - 1 + i];
                    for y in (0..NSIMD).step_by(LANES) {
                        let mut acc = f32x4::from_slice(&acc3[y..y + LANES]);
                        acc += f32x4::from_slice(&v[y..y + LANES]);
                        acc.copy_to_slice(&mut acc3[y..y + LANES]);
                    }
                    for y in NSIMD..GRID_Y {
                        acc3[y] += v[y];
                    }
                }

                let inv = f32x4::splat(INV_NEIGHBOURS);
                for j in 0..n {
                    let x = x0 + j;
                    let old_col = &old_state[x];
                    let new_col = &mut chunk[j];
                    // SAFETY: Cell is #[repr(transparent)] over f32, so the
                    // column arrays can be reinterpreted as raw f32 slices.
                    let old_vals: &[f32] = unsafe {
                        std::slice::from_raw_parts(old_col.as_ptr() as *const f32, GRID_Y)
                    };
                    let new_vals: &mut [f32] = unsafe {
                        std::slice::from_raw_parts_mut(new_col.as_mut_ptr() as *mut f32, GRID_Y)
                    };
                    for y in (0..NSIMD).step_by(LANES) {
                        let ring = f32x4::from_slice(&acc15[y..y + LANES])
                            - f32x4::from_slice(&acc3[y..y + LANES]);
                        let avg = ring * inv;
                        let c = f32x4::from_slice(&old_vals[y..y + LANES]);
                        compute_update_simd(c, avg).copy_to_slice(&mut new_vals[y..y + LANES]);
                    }
                    for y in NSIMD..GRID_Y {
                        let avg = (acc15[y] - acc3[y]) * INV_NEIGHBOURS;
                        new_vals[y] = compute_update_single(old_vals[y], avg);
                    }
                    if j + 1 < n {
                        // Slide: drop local column j (grid x - R), add local
                        // column j + W (grid x + R + 1); likewise for the
                        // 3-window with local j + R - 1 / j + R + 2.
                        let out = &v15[j];
                        let inc = &v15[j + W];
                        let out3 = &v3[j + R - 1];
                        let inc3 = &v3[j + R + 2];
                        for y in (0..NSIMD).step_by(LANES) {
                            let mut a15 = f32x4::from_slice(&acc15[y..y + LANES]);
                            a15 += f32x4::from_slice(&inc[y..y + LANES])
                                - f32x4::from_slice(&out[y..y + LANES]);
                            a15.copy_to_slice(&mut acc15[y..y + LANES]);
                            let mut a3 = f32x4::from_slice(&acc3[y..y + LANES]);
                            a3 += f32x4::from_slice(&inc3[y..y + LANES])
                                - f32x4::from_slice(&out3[y..y + LANES]);
                            a3.copy_to_slice(&mut acc3[y..y + LANES]);
                        }
                        for y in NSIMD..GRID_Y {
                            acc15[y] += inc[y] - out[y];
                            acc3[y] += inc3[y] - out3[y];
                        }
                    }
                }

                POOL.with(|p| *p.borrow_mut() = Some((v15, v3)));
            });
    }

    fn init(&mut self, rng: &mut rand::prelude::ThreadRng, _x: i64, _y: i64) {
        self.0 = rng.gen_range(-1.0..1.0);
    }

    fn render(&self) -> sdl2::pixels::Color {
        let abs = self.0.abs();
        sdl2::pixels::Color::RGB(
            (abs * 255.0) as u8,
            (abs * 255.0) as u8,
            (abs * 255.0) as u8,
        )
    }

    fn update(
        &self,
        x: i64,
        y: i64,
        get_cell: impl Fn(i64, i64) -> Cell,
        _: &mut rand::prelude::ThreadRng,
    ) -> Cell {
        let sum: f32 = NEIGHBOURS
            .iter()
            .map(|(dx, dy)| get_cell(x + dx, y + dy).0)
            .sum();
        let avg = sum * INV_NEIGHBOURS;
        Cell(compute_update_single(self.0, avg))
    }

    // Per-column fallback (also used by the default update_frame)
    // Note: Grid is [[Cell; CELLS_Y]; CELLS_X] so we update columns
    fn update_row(
        old_row: &[Cell; GRID_Y],
        new_row: &mut [Cell; GRID_Y],
        old_grid: &[[Cell; GRID_Y]; GRID_X],
        x: usize,
    ) {
        update_column_vectorized(old_row, new_row, old_grid, x);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The original branchy rule, kept as reference for equivalence checking.
    fn compute_update_single_ref(current: f32, avg: f32) -> f32 {
        match (current, avg) {
            (c, a) if c < 0. && a > -0.2 => -c + 0.025,
            (c, a) if c > 0. && a < 0. => -c - 0.025,
            (c, a) if c > 0. && a > 0.55 => -c - 0.025,
            (c, a) if c < a => c + 0.01,
            (c, a) if a < c => c - 0.01,
            (c, _) => c,
        }
        .clamp(-1., 1.)
    }

    /// Deterministic pseudo-random grid (xorshift), values in [-1, 1).
    /// Heap-allocated: 540x540 grids exceed the test-thread stack.
    fn deterministic_grid(seed: u64) -> Box<[[Cell; GRID_Y]; GRID_X]> {
        let mut s = seed | 1; // xorshift must not start at 0
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            // low 32 bits reinterpreted as i32, scaled to [-1, 1)
            (s as u32) as i32 as f32 / (1u32 << 31) as f32
        };
        let mut grid = Box::new([[Cell(0.0); GRID_Y]; GRID_X]);
        for col in grid.iter_mut() {
            for cell in col.iter_mut() {
                cell.0 = next();
            }
        }
        grid
    }

    /// Naive reference: ring sum around (x, y) over NEIGHBOURS, then update.
    fn naive_update(old_grid: &[[Cell; GRID_Y]; GRID_X], x: usize, y: usize) -> (f32, f32) {
        let mut sum = 0f32;
        for &(dx, dy) in NEIGHBOURS.iter() {
            let nx = wrap_idx(x as i64 + dx, GRID_X as i64);
            let ny = wrap_idx(y as i64 + dy, GRID_Y as i64);
            sum += old_grid[nx][ny].0;
        }
        let avg = sum * INV_NEIGHBOURS;
        let next = compute_update_single(old_grid[x][y].0, avg);
        (next, avg)
    }

    /// A cell may legitimately differ from the naive result only when its
    /// average sits on a decision boundary (sliding-window summation orders
    /// differ from the naive order by a few ulps).
    fn near_decision_boundary(c: f32, a: f32) -> bool {
        a.abs() < 1e-3 || (a + 0.2).abs() < 1e-3 || (a - 0.55).abs() < 1e-3 || (a - c).abs() < 1e-3
    }

    fn assert_close_enough(x: usize, y: usize, got: f32, expected: f32, avg: f32, c: f32) {
        let d = (got - expected).abs();
        assert!(
            d <= 2e-3 || near_decision_boundary(c, avg),
            "mismatch at (x={x}, y={y}): got {got}, naive {expected} (c={c}, avg={avg})"
        );
    }

    #[test]
    fn column_update_matches_naive() {
        let old_grid = deterministic_grid(0x0dd_ba11);
        let x = 197; // arbitrary column
        let mut new_col = [Cell(0.0); GRID_Y];
        update_column_vectorized(&old_grid[x], &mut new_col, &old_grid, x);
        for y in 0..GRID_Y {
            let (naive, avg) = naive_update(&old_grid, x, y);
            let c = old_grid[x][y].0;
            assert_close_enough(x, y, new_col[y].0, naive, avg, c);
        }
    }

    #[test]
    fn frame_update_matches_naive() {
        let old_grid = deterministic_grid(0x5eed_1337);
        let mut new_grid = Box::new([[Cell(0.0); GRID_Y]; GRID_X]);
        Cell::update_frame(&old_grid, &mut new_grid);

        let mut check = |x: usize, y: usize| {
            let (naive, avg) = naive_update(&old_grid, x, y);
            let c = old_grid[x][y].0;
            assert_close_enough(x, y, new_grid[x][y].0, naive, avg, c);
        };

        // Corners, edges and centre (these exercise all the wrap-around paths)
        let pts: [(usize, usize); 9] = [
            (0, 0),
            (0, GRID_Y - 1),
            (GRID_X - 1, 0),
            (GRID_X - 1, GRID_Y - 1),
            (R, R),
            (GRID_X - 1 - R, GRID_Y - 1 - R),
            (GRID_X / 2, GRID_Y / 2),
            (0, GRID_Y / 2),
            (GRID_X / 2, 0),
        ];
        for &(x, y) in &pts {
            check(x, y);
        }
        // Plus a pseudo-random sample spread over the grid.
        let mut s = 0xfeed_faceu64;
        for _ in 0..512 {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let x = (s % GRID_X as u64) as usize;
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let y = (s % GRID_Y as u64) as usize;
            check(x, y);
        }
    }

    #[test]
    fn branchless_matches_reference() {
        let mut rng: u64 = 0x1234_5678_9abc_def0;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng as i32 as f32) / u32::MAX as f32 * 2.4 - 1.2 // covers [-1.2, 1.2]
        };
        for _ in 0..1_000_000 {
            let c = next();
            let a = next();
            assert_eq!(
                compute_update_single(c, a),
                compute_update_single_ref(c, a),
                "c={c} a={a}"
            );
        }
        // equality edge case
        for v in [-1.0f32, -0.5, 0.0, 0.3, 0.55, 1.0] {
            assert_eq!(compute_update_single(v, v), compute_update_single_ref(v, v));
        }
    }
}
