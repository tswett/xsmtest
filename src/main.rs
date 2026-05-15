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

mod mixers;
mod prng;

use clap::{Parser, ValueEnum};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::ops::{Add, AddAssign};

use crate::mixers::{Mixer, MultiplyInvMut, MultiplyMut, Mutation, XorshiftRightMut};
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

fn print_hamming_dist_stats(stats: SampleStats) {
    println!(
        "Mean Hamming distance: {:9.6} (Z = {:6.2})", stats.sample_mean, stats.mean_z);
    println!(
        "Standard deviation   : {:9.6} (Z = {:6.2})", stats.sample_stddev, stats.stddev_z);
}

fn avalanche_test_inner(mut prng: PRNG, mixer: &dyn Mixer, samples: u64)
    -> RawStats
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

fn avalanche_test(prng: PRNG, mixer: &dyn Mixer, samples: u64) -> SampleStats
{
    let stats: RawStats = BatchMaker { prng, samples }
        .par_bridge()
        .map(|batch_info|
            avalanche_test_inner(batch_info.prng, mixer, batch_info.samples))
        .reduce(RawStats::default, |a, b| a + b);

    let expected_mean: f64 = 32.0;
    let expected_stddev: f64 = 4.0;

    stats.sample_stats(expected_mean, expected_stddev)
}

fn run_avalanche_test_results(prng: PRNG, name: &str, mixer: &dyn Mixer, samples: u64)
    -> SampleStats
{
    println!("Testing {}:", name);

    let stats = avalanche_test(prng, mixer, samples);

    print_hamming_dist_stats(stats);
    println!();

    stats
}

fn run_avalanche_test(prng: PRNG, name: &str, mixer: &dyn Mixer, samples: u64)
    -> ()
{
    run_avalanche_test_results(prng, name, mixer, samples);
}

#[derive(Clone, Copy)]
struct MutationInfo {
    code_start: &'static str,
    operand: u32,
    code_end: &'static str,
    badness: f64,
}

fn run_mutation_test_on<'a, M: Mutation<'a>>
    (mut prng: PRNG, name: &str, mixer: &'a dyn Mixer, samples: u64)
    -> (MutationInfo, MutationInfo)
{
    let mut best = MutationInfo {
        code_start: "",
        operand: 0,
        code_end: "",
        badness: f64::MAX,
    };
    let mut worst = MutationInfo {
        code_start: "",
        operand: 0,
        code_end: "",
        badness: -1.0,
    };

    for operand in M::RANGE {
        let mutated = M::new(mixer, operand);

        let stats = avalanche_test(prng.get_prng(), &mutated, samples);
        let badness = stats.mean_z.abs().max(stats.stddev_z.abs());

        println!("{}; {}{}{}:", name, M::CODE_START, operand, M::CODE_END);
        print_hamming_dist_stats(stats);
        println!();

        let mutation_info = MutationInfo {
            code_start: M::CODE_START,
            operand,
            code_end: M::CODE_END,
            badness,
        };

        if badness < best.badness { best = mutation_info }
        if badness > worst.badness { worst = mutation_info }
    }

    (best, worst)
}

fn run_mutation_test(mut prng: PRNG, name: &str, mixer: &dyn Mixer, samples: u64)
{
    let base_stats = run_avalanche_test_results(prng.get_prng(), name, mixer, samples);
    let base_badness = base_stats.mean_z.abs().max(base_stats.stddev_z.abs());

    let mut best = MutationInfo {
        code_start: "",
        operand: 0,
        code_end: "",
        badness: f64::MAX,
    };
    let mut worst = MutationInfo {
        code_start: "",
        operand: 0,
        code_end: "",
        badness: -1.0,
    };

    let (best_xorshift_right, worst_xorshift_right) =
        run_mutation_test_on::<XorshiftRightMut>(prng.get_prng(), name, mixer, samples);

    if best_xorshift_right.badness < best.badness { best = best_xorshift_right }
    if worst_xorshift_right.badness > worst.badness { worst = worst_xorshift_right }

    let (best_multiply, worst_multiply) =
        run_mutation_test_on::<MultiplyMut>(prng.get_prng(), name, mixer, samples);

    if best_multiply.badness < best.badness { best = best_multiply }
    if worst_multiply.badness > worst.badness { worst = worst_multiply }

    let (best_multiply_inv, worst_multiply_inv) =
        run_mutation_test_on::<MultiplyInvMut>(prng.get_prng(), name, mixer, samples);

    if best_multiply_inv.badness < best.badness { best = best_multiply_inv }
    if worst_multiply_inv.badness > worst.badness { worst = worst_multiply_inv }

    println!("Baseline: {:.2}", base_badness);
    println!(
        "Best multiply: {}{}{} ({:.2})",
        best_multiply.code_start,
        best_multiply.operand,
        best_multiply.code_end,
        best_multiply.badness);
    println!(
        "Best: {}{}{} ({:.2})",
        best.code_start,
        best.operand,
        best.code_end,
        best.badness);
    println!(
        "Worst: {}{}{} ({:.2})",
        worst.code_start,
        worst.operand,
        worst.code_end,
        worst.badness);
    println!();
}

#[derive(Clone, ValueEnum)]
enum TestType {
    Avalanche,
    Mutation,
}

#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t = 1 << 17)]
    samples: u64,

    #[arg(long, default_value = "all")]
    mixer: String,

    #[arg(long, value_enum, default_value_t = TestType::Avalanche)]
    test: TestType,
}

fn main() {
    let mut prng = PRNG::from_seed(0);

    let args = Args::parse();

    println!(
        "Running avalanche tests. Theoretically, the mean should be 32 \
        and the standard deviation should be 4.");
    println!();
    println!("{} samples.", args.samples);
    println!();

    let run_test = match args.test {
        TestType::Avalanche => run_avalanche_test,
        TestType::Mutation => run_mutation_test,
    };

    if args.mixer == "all" {
        for m in mixers::MIXERS {
            run_test(prng.get_prng(), m.name, m.func, args.samples);
        }
    } else {
        match mixers::MIXERS.iter().find(|m| m.name == args.mixer) {
            Some(m) => run_test(prng, m.name, m.func, args.samples),
            None => panic!("Unknown mixer: {}", args.mixer),
        }
    }
}
