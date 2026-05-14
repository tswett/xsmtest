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
use prng::PRNG;
use std::ops::{Add, AddAssign};

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
        let n = self.count as f64;

        let sample_mean = (self.sum as f64) / (self.count as f64);

        // sum_sq/n - (sum/n)^2, rewritten to avoid floating point subtraction
        let sample_variance =
            ((self.sum_sq * self.count - self.sum * self.sum) as f64) / (n * n);

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

fn avalanche_test_inner(mut prng: PRNG, mix: &dyn Fn(u64) -> u64, samples: u64)
    -> RawStats
{
    let mut stats = RawStats::default();

    for _ in 0..samples {
        let input1: u64 = prng.get_number();

        let output1 = mix(input1);

        for bit in 0..64 {
            let input2 = input1 ^ (1u64 << bit);
            let output2 = mix(input2);

            let distance = (output1 ^ output2).count_ones() as i64;

            stats.tally(distance);
        }
    }

    stats
}

fn avalanche_test(mut prng: PRNG, mix: &dyn Fn(u64) -> u64, samples: u64) -> SampleStats
{
    const BATCH_SIZE: u64 = 4096;

    let mut samples_to_go = samples;
    let mut stats = RawStats::default();

    while samples_to_go > 0 {
        let batch_samples = samples_to_go.max(BATCH_SIZE);
        samples_to_go -= batch_samples;

        stats += avalanche_test_inner(prng.get_prng(), mix, batch_samples);
    }

    let expected_mean: f64 = 32.0;
    let expected_stddev: f64 = 4.0;

    stats.sample_stats(expected_mean, expected_stddev)
}

fn run_avalanche_test_results(
    prng: PRNG, name: &str, mixer: &dyn Fn(u64) -> u64, samples: u64)
    -> SampleStats
{
    println!("Testing {}:", name);

    let stats = avalanche_test(prng, mixer, samples);

    print_hamming_dist_stats(stats);
    println!();

    stats
}

fn run_avalanche_test(prng: PRNG, name: &str, mixer: &dyn Fn(u64) -> u64, samples: u64)
    -> ()
{
    run_avalanche_test_results(prng, name, mixer, samples);
}

#[derive(Clone)]
struct Mutation {
    name: String,
    badness: f64,
}

fn run_mutation_test(
    mut prng: PRNG, name: &str, mixer: &dyn Fn(u64) -> u64, samples: u64)
{
    let base_stats = run_avalanche_test_results(prng.get_prng(), name, mixer, samples);
    let base_badness = base_stats.mean_z.abs().max(base_stats.stddev_z.abs());

    let mut best = Mutation { name: "".to_string(), badness: f64::MAX };
    let mut best_multiply = Mutation { name: "".to_string(), badness: f64::MAX };
    let mut worst = Mutation { name: "".to_string(), badness: -1.0 };

    for n in 1..64 {
        let mut_name = format!("x ^= x >> {n}");

        let mutated = |mut x: u64| { x = mixer(x); x ^ (x >> n) };

        let stats = avalanche_test(prng.get_prng(), &mutated, samples);
        let badness = stats.mean_z.abs().max(stats.stddev_z.abs());

        println!("{name}; {mut_name}:");
        print_hamming_dist_stats(stats);
        println!();

        if badness < best.badness { best = Mutation { name: mut_name.clone(), badness: badness } }
        if badness > worst.badness { worst = Mutation { name: mut_name.clone(), badness: badness } }
    }

    for n in 0..64 {
        let mut_name = format!("x += 1 << {n}");

        let mutated = |x: u64| mixer(x).wrapping_add(1 << n);

        let stats = avalanche_test(prng.get_prng(), &mutated, samples);
        let badness = stats.mean_z.abs().max(stats.stddev_z.abs());

        println!("{name}; {mut_name}:");
        print_hamming_dist_stats(stats);
        println!();

        if badness < best.badness { best = Mutation { name: mut_name.clone(), badness: badness } }
        if badness > worst.badness { worst = Mutation { name: mut_name.clone(), badness: badness } }
    }

    for n in 0..63 {
        let mut_name = format!("x -= 1 << {n}");

        let mutated = |x: u64| mixer(x).wrapping_sub(1 << n);

        let stats = avalanche_test(prng.get_prng(), &mutated, samples);
        let badness = stats.mean_z.abs().max(stats.stddev_z.abs());

        println!("{name}; {mut_name}:");
        print_hamming_dist_stats(stats);
        println!();

        if badness < best.badness { best = Mutation { name: mut_name.clone(), badness: badness } }
        if badness > worst.badness { worst = Mutation { name: mut_name.clone(), badness: badness } }
    }

    for n in 1..64 {
        let mut_name = format!("x *= 1 + (1 << {n})");

        let mutated = |mut x: u64| { x = mixer(x); x.wrapping_mul(1 + (1 << n)) };

        let stats = avalanche_test(prng.get_prng(), &mutated, samples);
        let badness = stats.mean_z.abs().max(stats.stddev_z.abs());

        println!("{name}; {mut_name}:");
        print_hamming_dist_stats(stats);
        println!();

        if badness < best_multiply.badness { best_multiply = Mutation { name: mut_name.clone(), badness: badness } }
        if badness > worst.badness { worst = Mutation { name: mut_name.clone(), badness: badness } }
    }

    if best_multiply.badness < best.badness { best = best_multiply.clone() }

    println!("Baseline: {:.2}", base_badness);
    println!("Best multiply: {} ({:.2})", best_multiply.name, best_multiply.badness);
    println!("Best: {} ({:.2})", best.name, best.badness);
    println!("Worst: {} ({:.2})", worst.name, worst.badness);
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
            run_test(prng.get_prng(), m.name, &m.func, args.samples);
        }
    } else {
        match mixers::MIXERS.iter().find(|m| m.name == args.mixer) {
            Some(m) => run_test(prng, m.name, &m.func, args.samples),
            None => panic!("Unknown mixer: {}", args.mixer),
        }
    }
}
