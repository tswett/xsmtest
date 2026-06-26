// Copyright 2026 Sophie Swett
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// 
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use clap::ValueEnum;
use rayon::iter::{ParallelBridge, ParallelIterator};
use pyo3::prelude::pymodule;
use std::ops::{Add, AddAssign, Range};

use crate::mixer::Mixer;
use crate::prng::PRNG;

#[derive(Clone, Copy, Default)]
struct RawStats {
    count: i64,
    sum: i64,
    sum_sq: i64,
}

impl RawStats {
    fn tally(&mut self, value: i64) {
        self.count += 1;
        self.sum += value;
        self.sum_sq += value * value;
    }

    fn sample_stats(self, expected_mean: f64, expected_stddev: f64) -> SampleStats {
        let n_i128: i128 = self.count as i128;
        let sum_i128: i128 = self.sum as i128;
        let sum_sq_i128: i128 = self.sum_sq as i128;

        let n = self.count as f64;

        let sample_mean = (self.sum as f64) / (self.count as f64);

        // sum_sq/n - (sum/n)^2, rewritten to avoid floating point subtraction
        let sample_variance =
            ((sum_sq_i128 * n_i128 - sum_i128 * sum_i128) as f64) / (n * n);

        let sample_stddev = sample_variance.sqrt();

        let mean_z = (sample_mean - expected_mean) / (expected_stddev / n.sqrt());

        // this is an approximation, hopefully not a bad one
        let stddev_z =
            (sample_stddev - expected_stddev) / (expected_stddev / (2.0 * n).sqrt());

        SampleStats { sample_mean, sample_stddev, mean_z, stddev_z }
    }
}

impl Add for RawStats {
    type Output = RawStats;

    fn add(self, rhs: RawStats) -> RawStats {
        RawStats {
            count: self.count + rhs.count,
            sum: self.sum + rhs.sum,
            sum_sq: self.sum_sq + rhs.sum_sq,
        }
    }
}

impl AddAssign for RawStats {
    fn add_assign(&mut self, rhs: RawStats) {
        *self = *self + rhs;
    }
}

#[derive(Clone, Copy)]
struct SampleStats {
    sample_mean: f64,
    sample_stddev: f64,
    mean_z: f64,
    stddev_z: f64
}

pub const DEFAULT_SAMPLES: u64 = 200000;
pub const DEFAULT_SEED: u64 = 0;

pub struct MixerTestContext<'a> {
    pub prng: PRNG,

    pub name: &'a str,
    pub mixer: &'a Mixer,

    pub samples: u64,
}

impl<'a> MixerTestContext<'a> {
    fn split(&mut self) -> Self {
        MixerTestContext {
            prng: self.prng.get_prng(),
            ..*self
        }
    }
}

pub trait MixerTest {
    fn run_test(&self, ctx: MixerTestContext);
}

const BATCH_SIZE: u64 = 4096;

struct BatchMaker {
    prng: PRNG,
    samples: u64,
}

struct BatchInfo {
    prng: PRNG,
    samples: u64,
}

impl Iterator for BatchMaker {
    type Item = BatchInfo;

    fn next(&mut self) -> Option<BatchInfo> {
        if self.samples == 0 {
            None
        } else {
            let batch_info = BatchInfo {
                prng: self.prng.get_prng(),
                samples: self.samples.min(BATCH_SIZE),
            };

            self.samples -= batch_info.samples;

            Some(batch_info)
        }
    }
}

pub struct Avalanche;

impl Avalanche {
    fn test_inner(
        mut prng: PRNG, mixer: &Mixer, samples: u64, bit_range: Range<i32>)
        -> RawStats
    {
        let mut stats = RawStats::default();

        for _ in 0..samples {
            let input1: u64 = prng.get_number();

            let output1 = mixer.mix(input1);

            for bit in bit_range.clone() {
                let input2 = input1 ^ (1u64 << bit);
                let output2 = mixer.mix(input2);

                let distance = (output1 ^ output2).count_ones() as i64;

                stats.tally(distance);
            }
        }

        stats
    }

    fn avalanche_test(ctx: MixerTestContext, bit_range: Range<i32>)
        -> SampleStats
    {
        let stats: RawStats = BatchMaker { prng: ctx.prng, samples: ctx.samples }
            .par_bridge()
            .map(|batch_info|
                Self::test_inner(
                    batch_info.prng, ctx.mixer, batch_info.samples, bit_range.clone()))
            .reduce(RawStats::default, |a, b| a + b);

        let expected_mean: f64 = 32.0;
        let expected_stddev: f64 = 4.0;

        stats.sample_stats(expected_mean, expected_stddev)
    }
}

impl MixerTest for Avalanche {
    fn run_test(&self, ctx: MixerTestContext) {
        println!("Testing {} with {} samples:", ctx.name, ctx.samples);

        let stats = Self::avalanche_test(ctx, 0..64);

        println!("Mean Hamming distance: {:9.6} (Z = {:6.2})",
            stats.sample_mean, stats.mean_z);
        println!("Standard deviation   : {:9.6} (Z = {:6.2})",
            stats.sample_stddev, stats.stddev_z);
        println!();
    }
}

pub struct AvalancheBitwise;

impl MixerTest for AvalancheBitwise {
    fn run_test(&self, mut ctx: MixerTestContext) {
        println!("Testing {} with {} samples:", ctx.name, ctx.samples);

        for bit in 0..64 {
            let stats = Avalanche::avalanche_test(ctx.split(), bit..bit+1);

            println!(
                "bit {:2}:    Mean: {:9.6}    Z: {:8.2}    Std dev: {:9.6}    Z: {:8.2}",
                bit,
                stats.sample_mean,
                stats.mean_z,
                stats.sample_stddev,
                stats.stddev_z);
        }

        println!();
    }
}

struct SARawStats {
    input_count: Box<[u64; 64]>,
    flip_count: Box<[[u64; 64]; 64]>,
}

impl SARawStats {
    fn tally(&mut self, bit: usize, difference: u64) {
        self.input_count[bit] += 1;

        for i in 0..64 {
            if difference & (1 << i) != 0 {
                self.flip_count[bit][i] += 1;
            }
        }
    }

    fn calculate(&self) -> SACalcStats {
        let mut n: f64 = 0.0;
        let mut sum_p: f64 = 0.0;
        let mut sum_p_sq: f64 = 0.0;
        let mut min_p = f64::MAX;
        let mut max_p = f64::MIN;

        for bit in 0..64 {
            if self.input_count[bit] == 0 {
                continue;
            }

            for i in 0..64 {
                let p =
                    (self.flip_count[bit][i] as f64) /
                    (self.input_count[bit] as f64);

                n += 1.0;
                sum_p += p;
                sum_p_sq += p*p;
                if p < min_p { min_p = p }
                if p > max_p { max_p = p }
            }
        }

        let mean_p = sum_p / n;
        let stddev_p = (sum_p_sq / n - mean_p * mean_p).sqrt();

        SACalcStats {
            min_p,
            max_p,
            mean_p,
            stddev_p,
        }
    }
}

impl Default for SARawStats {
    fn default() -> Self {
        SARawStats {
            input_count: Box::new([0; 64]),
            flip_count: Box::new([[0; 64]; 64]),
        }
    }
}

impl Add for SARawStats {
    type Output = SARawStats;

    fn add(self, rhs: SARawStats) -> SARawStats {
        let mut total: SARawStats = self;

        for i in 0..64 {
            total.input_count[i] += rhs.input_count[i];

            for j in 0..64 {
                total.flip_count[i][j] += rhs.flip_count[i][j];
            }
        }

        total
    }
}

struct SACalcStats {
    min_p: f64,
    max_p: f64,
    mean_p: f64,
    stddev_p: f64,
}

pub struct StrictAvalanche;

impl StrictAvalanche {
    fn test_inner(mut prng: PRNG, mixer: &Mixer, samples: u64)
        -> SARawStats
    {
        let mut stats = SARawStats::default();

        for _ in 0..samples {
            let input1: u64 = prng.get_number();

            let output1 = mixer.mix(input1);

            for bit in 0..64 {
                let input2 = input1 ^ (1u64 << bit);
                let output2 = mixer.mix(input2);

                stats.tally(bit, output1 ^ output2);
            }
        }

        stats
    }
}

impl MixerTest for StrictAvalanche {
    fn run_test(&self, ctx: MixerTestContext) {
        println!("Testing {} with {} samples:", ctx.name, ctx.samples);

        let stats: SARawStats = BatchMaker { prng: ctx.prng, samples: ctx.samples }
            .par_bridge()
            .map(|batch_info|
                Self::test_inner(batch_info.prng, ctx.mixer, batch_info.samples))
            .reduce(SARawStats::default, |a, b| a + b);

        let calc_stats = stats.calculate();

        // TODO: calculate the Z-scores correctly
        println!("Lowest probability : {:9.6} (Z = ?)", calc_stats.min_p);
        println!("Highest probability: {:9.6} (Z = ?)", calc_stats.max_p);
        println!("Mean probability   : {:9.6} (Z = ?)", calc_stats.mean_p);
        println!("Std dev probability: {:9.6} (Z = ?)", calc_stats.stddev_p);

        // TODO: show a histogram of the Z-scores

        // TODO: show a heatmap of the Z-scores

        println!();
    }
}

pub struct Powers;

impl MixerTest for Powers {
    fn run_test(&self, ctx: MixerTestContext) {
        println!("Calling {} with powers of 2:", ctx.name);

        let mut last_output: Option<u64> = None;

        for i in 0..64 {
            let output = ctx.mixer.mix(1 << i);

            let distance = last_output.map(|last_output| {
                let last_output_masked = last_output & !(1 << 63);
                let output_shifted = output >> 1;
                (last_output_masked ^ output_shifted).count_ones()
            });

            print!("{:>2}  ", i);

            for b in (0..64).rev() {
                let color_code = if output & (1 << b) != 0 {
                    "\x1b[7m:"
                } else {
                    "\x1b[0m."
                };

                print!("{}", color_code);
            }

            print!("\x1b[0m");

            print!("  ");
            if let Some(distance) = distance {
                println!("{:>2}", distance)
            } else {
                println!()
            }

            last_output = Some(output);
        }
    }
}

pub struct Shift;

impl MixerTest for Shift {
    fn run_test(&self, mut ctx: MixerTestContext) {
        println!("Running bit shift test on {} with {} samples:",
            ctx.name, ctx.samples);

        let mut histogram: [u64; 64] = [0; 64];

        for _ in 0..ctx.samples {
            let input = ctx.prng.get_number() & !(1 << 63);

            let difference =
                (ctx.mixer.mix(input) << 1) ^ (ctx.mixer.mix(input << 1) & !1);

            let distance = difference.count_ones();

            histogram[distance as usize] += 1;
        }

        let segment_size = (histogram.iter().max().unwrap() / 60).max(1);

        for bucket in 0..64 {
            print!("{:>2}: ", bucket);

            let bar_size = (histogram[bucket] + segment_size - 1) / segment_size;

            for _ in 0..bar_size { print!("\u{2588}"); };

            println!(" ({})", histogram[bucket]);
        }
    }
}

pub struct Z3;

impl MixerTest for Z3 {
    fn run_test(&self, ctx: MixerTestContext) {
        let samples = ctx.samples.min(64);

        println!("Using Z3 to solve {} with {} samples...", ctx.name, samples);
        println!();

        let solver = z3::Solver::new();

        let p_s1 = z3::ast::BV::new_const("S1", 64);
        solver.assert(p_s1.bvuge(1));
        solver.assert(p_s1.bvule(64));
        let p_m1 = z3::ast::BV::new_const("M1", 64);
        let p_s2 = z3::ast::BV::new_const("S2", 64);
        solver.assert(p_s2.bvuge(1));
        solver.assert(p_s2.bvule(64));
        let p_m2 = z3::ast::BV::new_const("M2", 64);
        let p_s3 = z3::ast::BV::new_const("S3", 64);
        solver.assert(p_s3.bvuge(1));
        solver.assert(p_s3.bvule(64));

        for i in 0..samples {
            let mut x = z3::ast::BV::from_u64(1 << i, 64);

            x = x.bvxor(x.bvlshr(&p_s1));

            x = x.bvmul(&p_m1);
            x = x.bvxor(x.bvlshr(&p_s2));

            x = x.bvmul(&p_m2);
            x = x.bvxor(x.bvlshr(&p_s3));

            solver.assert(x.eq(z3::ast::BV::from_u64(ctx.mixer.mix(1 << i), 64)));
        }

        let status = solver.check();

        match status {
            z3::SatResult::Sat => {
                let model = solver.get_model().unwrap();

                let v_s1 = model.eval(&p_s1, true).unwrap().as_u64().unwrap();
                let v_m1 = model.eval(&p_m1, true).unwrap().as_u64().unwrap();
                let v_s2 = model.eval(&p_s2, true).unwrap().as_u64().unwrap();
                let v_m2 = model.eval(&p_m2, true).unwrap().as_u64().unwrap();
                let v_s3 = model.eval(&p_s3, true).unwrap().as_u64().unwrap();

                println!("Found a solution:");
                println!();
                println!("x ^= x >> {};", v_s1);
                println!("x *= 0x{:016x};", v_m1);
                println!("x ^= x >> {};", v_s2);
                println!("x *= 0x{:016x};", v_m2);
                println!("x ^= x >> {};", v_s3);
            }
            z3::SatResult::Unsat =>
                println!("Search complete. There are no solutions."),
            z3::SatResult::Unknown =>
                println!("Search interrupted."),
        }

        println!();
    }
}

#[derive(Clone, ValueEnum)]
pub enum TestType {
    Avalanche,
    AvalancheBitwise,
    Powers,
    Shift,
    StrictAvalanche,
    Z3,
}

#[pymodule(name = "mixertests", module = "xsmtest")]
pub mod py_mixertests {
    use pyo3::prelude::pyfunction;

    use crate::mixer::PyMixerDef;
    use crate::prng::PRNG;
    use super::{
        Avalanche, DEFAULT_SAMPLES, DEFAULT_SEED, MixerTestContext, MixerTest
    };

    #[pyfunction]
    #[pyo3(signature = (mixer, seed=DEFAULT_SEED, samples=DEFAULT_SAMPLES))]
    fn run_avalanche(mixer: &mut PyMixerDef, seed: u64, samples: u64) {
        let ctx = MixerTestContext {
            prng: PRNG::from_seed(seed),
            name: &mixer.name(),
            mixer: &mixer.compile(),
            samples,
        };

        Avalanche.run_test(ctx);
    }
}
