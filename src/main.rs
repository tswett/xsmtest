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

fn avalanche_test<F>(mut prng: PRNG, mix: F, samples: u64) -> (f64, f64, f64, f64)
where
    F: Fn(u64) -> u64,
{
    let mut n: u64 = 0;
    let mut sum: u64 = 0;
    let mut sum_sq: u64 = 0;

    for _ in 0..samples {
        let input1: u64 = prng.get_number();

        let output1 = mix(input1);

        for bit in 0..64 {
            let input2 = input1 ^ (1u64 << bit);
            let output2 = mix(input2);

            let distance = (output1 ^ output2).count_ones() as u64;

            n += 1;
            sum += distance;
            sum_sq += distance * distance;
        }
    }

    let mean = (sum as f64) / (n as f64);
    let variance = (sum_sq as f64) / (n as f64) - mean * mean;
    let stddev = variance.sqrt();

    let mean_sample_sd = 4.0 / (n as f64).sqrt();
    let stddev_sample_sd = 4.0 / (2.0 * (n as f64)).sqrt();

    let mean_z = (mean - 32.0) / mean_sample_sd;
    let stddev_z = (stddev - 4.0) / stddev_sample_sd;

    (mean, stddev, mean_z, stddev_z)
}

fn run_avalanche_test_results(prng: PRNG, name: &str, mixer: fn(u64) -> u64, samples: u64)
    -> (f64, f64, f64, f64)
{
    println!("Testing {}:", name);

    let (mean, stddev, mean_z, stddev_z) = avalanche_test(prng, mixer, samples);

    println!("Mean Hamming distance: {:9.6} (Z = {:6.2})", mean, mean_z);
    println!("Standard deviation   : {:9.6} (Z = {:6.2})", stddev, stddev_z);
    println!();

    (mean, stddev, mean_z, stddev_z)
}

fn run_avalanche_test(prng: PRNG, name: &str, mixer: fn(u64) -> u64, samples: u64) -> () {
    run_avalanche_test_results(prng, name, mixer, samples);
}

#[derive(Clone)]
struct Mutation {
    name: String,
    badness: f64,
}

fn run_mutation_test(mut prng: PRNG, name: &str, mixer: fn(u64) -> u64, samples: u64) {
    let (_, _, base_mean_z, base_stddev_z) =
        run_avalanche_test_results(prng.get_prng(), name, mixer, samples);
    let base_badness = base_mean_z.abs().max(base_stddev_z.abs());

    let mut best = Mutation { name: "".to_string(), badness: f64::MAX };
    let mut best_multiply = Mutation { name: "".to_string(), badness: f64::MAX };
    let mut worst = Mutation { name: "".to_string(), badness: -1.0 };

    for n in 1..64 {
        let mut_name = format!("x ^= x >> {n}");

        let mutated = |mut x: u64| { x = mixer(x); x ^ (x >> n) };

        let (mean, stddev, mean_z, stddev_z) =
            avalanche_test(prng.get_prng(), mutated, samples);
        let badness = mean_z.abs().max(stddev_z.abs());

        println!("{name}; {mut_name}:");
        println!("Mean Hamming distance: {:9.6} (Z = {:6.2})", mean, mean_z);
        println!("Standard deviation   : {:9.6} (Z = {:6.2})", stddev, stddev_z);
        println!();

        if badness < best.badness { best = Mutation { name: mut_name.clone(), badness: badness } }
        if badness > worst.badness { worst = Mutation { name: mut_name.clone(), badness: badness } }
    }

    /*
    for n in 1..64 {
        let mut_name = format!("x ^= x << {n}");

        let mutated = |mut x: u64| { x = mixer(x); x ^ (x << n) };

        let (mean, stddev, mean_z, stddev_z) = avalanche_test(mutated, samples);
        let badness = mean_z.abs().max(stddev_z.abs());

        println!("{name}; {mut_name}:");
        println!("Mean Hamming distance: {:9.6} (Z = {:6.2})", mean, mean_z);
        println!("Standard deviation   : {:9.6} (Z = {:6.2})", stddev, stddev_z);
        println!();

        if badness < best.badness { best = Mutation { name: mut_name.clone(), badness: badness } }
        if badness > worst.badness { worst = Mutation { name: mut_name.clone(), badness: badness } }
    }
    */

    for n in 0..64 {
        let mut_name = format!("x += 1 << {n}");

        let mutated = |x: u64| mixer(x).wrapping_add(1 << n);

        let (mean, stddev, mean_z, stddev_z) =
            avalanche_test(prng.get_prng(), mutated, samples);
        let badness = mean_z.abs().max(stddev_z.abs());

        println!("{name}; {mut_name}:");
        println!("Mean Hamming distance: {:9.6} (Z = {:6.2})", mean, mean_z);
        println!("Standard deviation   : {:9.6} (Z = {:6.2})", stddev, stddev_z);
        println!();

        if badness < best.badness { best = Mutation { name: mut_name.clone(), badness: badness } }
        if badness > worst.badness { worst = Mutation { name: mut_name.clone(), badness: badness } }
    }

    for n in 0..63 {
        let mut_name = format!("x -= 1 << {n}");

        let mutated = |x: u64| mixer(x).wrapping_sub(1 << n);

        let (mean, stddev, mean_z, stddev_z) =
            avalanche_test(prng.get_prng(), mutated, samples);
        let badness = mean_z.abs().max(stddev_z.abs());

        println!("{name}; {mut_name}:");
        println!("Mean Hamming distance: {:9.6} (Z = {:6.2})", mean, mean_z);
        println!("Standard deviation   : {:9.6} (Z = {:6.2})", stddev, stddev_z);
        println!();

        if badness < best.badness { best = Mutation { name: mut_name.clone(), badness: badness } }
        if badness > worst.badness { worst = Mutation { name: mut_name.clone(), badness: badness } }
    }

    for n in 1..64 {
        let mut_name = format!("x *= 1 + (1 << {n})");

        let mutated = |mut x: u64| { x = mixer(x); x.wrapping_mul(1 + (1 << n)) };

        let (mean, stddev, mean_z, stddev_z) =
            avalanche_test(prng.get_prng(), mutated, samples);
        let badness = mean_z.abs().max(stddev_z.abs());

        println!("{name}; {mut_name}:");
        println!("Mean Hamming distance: {:9.6} (Z = {:6.2})", mean, mean_z);
        println!("Standard deviation   : {:9.6} (Z = {:6.2})", stddev, stddev_z);
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
            run_test(prng.get_prng(), m.name, m.func, args.samples);
        }
    } else {
        match mixers::MIXERS.iter().find(|m| m.name == args.mixer) {
            Some(m) => run_test(prng, m.name, m.func, args.samples),
            None => panic!("Unknown mixer: {}", args.mixer),
        }
    }
}
