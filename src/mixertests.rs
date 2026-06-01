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
use std::ops::{Add, AddAssign};

use crate::mixers::Mixer;
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

pub struct MixerTestContext<'a> {
    pub prng: PRNG,

    pub name: &'a str,
    pub mixer: &'a dyn Mixer,

    pub samples: u64,
}

impl<'a> MixerTestContext<'a> {
    fn split(&mut self) -> Self {
        MixerTestContext { prng: self.prng.get_prng(), ..*self }
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
    fn test_inner(mut prng: PRNG, mixer: &dyn Mixer, samples: u64) -> RawStats
    {
        let mut stats = RawStats::default();

        for _ in 0..samples {
            let input1: u64 = prng.get_number();

            let output1 = mixer.mix(input1);

            for bit in 0..64 {
                let input2 = input1 ^ (1u64 << bit);
                let output2 = mixer.mix(input2);

                let distance = (output1 ^ output2).count_ones() as i64;

                stats.tally(distance);
            }
        }

        stats
    }

    fn avalanche_test(ctx: MixerTestContext) -> SampleStats
    {
        let stats: RawStats = BatchMaker { prng: ctx.prng, samples: ctx.samples }
            .par_bridge()
            .map(|batch_info|
                Self::test_inner(batch_info.prng, ctx.mixer, batch_info.samples))
            .reduce(RawStats::default, |a, b| a + b);

        let expected_mean: f64 = 32.0;
        let expected_stddev: f64 = 4.0;

        stats.sample_stats(expected_mean, expected_stddev)
    }
}

impl MixerTest for Avalanche {
    fn run_test(&self, ctx: MixerTestContext) {
        println!("Testing {} with {} samples:", ctx.name, ctx.samples);

        let stats = Self::avalanche_test(ctx);

        println!("Mean Hamming distance: {:9.6} (Z = {:6.2})",
            stats.sample_mean, stats.mean_z);
        println!("Standard deviation   : {:9.6} (Z = {:6.2})",
            stats.sample_stddev, stats.stddev_z);
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

        let segment_size = histogram.iter().max().unwrap() / 60;

        for bucket in 0..64 {
            print!("{:>2}: ", bucket);

            let bar_size = (histogram[bucket] + segment_size - 1) / segment_size;

            for _ in 0..bar_size { print!("\u{2588}"); };

            println!(" ({})", histogram[bucket]);
        }
    }
}

#[derive(Clone, ValueEnum)]
pub enum TestType {
    Avalanche,
    Powers,
    Shift,
}
