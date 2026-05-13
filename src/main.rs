use clap::Parser;

// A totally trivial mixing function that literally does nothing.
fn trivial_mix(x: u64) -> u64 {
    x
}

// An atrocious mixing function which gets the mean Hamming distance right, but doesn't actually
// mix anything.
fn fake_ava_mix(x: u64) -> u64 {
    if (x >> 32).count_ones() % 2 == 1 {
        !x
    } else {
        x
    }
}

// Still an atrocious mixing function which gets both the mean and the standard deviation of the
// Hamming distance right, but doesn't actually mix anything. This one uses a carefully engineered
// relationship between the input bit that was flipped in the avalanche test and the number of
// output bits that flip as a result. Specifically:
//
// * at 12 input positions, a flip causes 37 output positions to flip,
// * at 20 input positions, a flip causes 33 output positions to flip,
// * at 20 input positions, a flip causes 31 output positions to flip, and
// * at 12 input positions, a flip causes 27 output positions to flip.
//
// This causes this "mixer" to get the mean and standard deviation correct on the nose (which is
// _better_ than a real mixer would perform).
fn faker_ava_mix(mut x: u64) -> u64 {
    if x.count_ones() % 2 == 1 {
        x ^= 0x00000000_FFFFFFFF;
    }
    if (x & 0x00000000_FFFFF000).count_ones() % 2 == 1 {
        x ^= 0x00000000_0000000F;
    }
    if (x & 0x000FFFFF_00000000).count_ones() % 2 == 1 {
        x ^= 0xF0000000_00000000;
    }

    x
}

// pi * 2^60, rounded to the nearest odd integer
const PI64: u64 = 0x3243F6A8885A308D;

// This gives us some avalanching, but not much.
fn terrible_pi_mix(x: u64) -> u64 {
    x.wrapping_mul(PI64)
}

// A little bit more avalanching, but not enough.
fn lousy_pi_mix(mut x: u64) -> u64 {
    x = x.wrapping_mul(PI64);
    x = x.rotate_right(32);

    x = x.wrapping_mul(PI64);

    x
}

fn xorshuffle_mix(rounds: u32, mut x: u64) -> u64 {
    for _ in 0..rounds {
        x ^= x >> 23;
        x ^= x << 17;
        x ^= x >> 13;
        x ^= x << 10;
    }

    x
}

// The venerable MurMurHash3 finalizer, taken from
// https://blog.teamleadnet.com/2012/08/murmurhash3-ultra-fast-hash-algorithm.html
fn murmurhash3_mix(mut x: u64) -> u64 {
    const M1: u64 = 0xFF51AFD7ED558CCD;
    const M2: u64 = 0xC4CEB9FE1A85EC53;

    x ^= x >> 33;

    x = x.wrapping_mul(M1);
    x ^= x >> 33;

    x = x.wrapping_mul(M2);
    x ^= x >> 33;

    x
}

fn extended_murmurhash3_mix(rounds: u32, mut x: u64) -> u64 {
    const M1: u64 = 0xFF51AFD7ED558CCD;
    const M2: u64 = 0xC4CEB9FE1A85EC53;

    x ^= x >> 33;

    for round in 0..rounds {
        x = x.wrapping_mul(if round % 2 == 0 { M1 } else { M2 });
        x ^= x >> 33;
    }

    x
}

// Pelle Evensen's NASAM, from
// https://mostlymangling.blogspot.com/2020/01/nasam-not-another-strange-acronym-mixer.html
fn nasam_mix(mut x: u64) -> u64 {
    const M1: u64 = 0x9E6C63D0676A9A99;
    const M2: u64 = 0x9E6D62D06F6A9A9B;

    x ^= x.rotate_right(25) ^ x.rotate_right(47);

    x = x.wrapping_mul(M1);
    x ^= (x >> 23) ^ (x >> 51);

    x = x.wrapping_mul(M2);
    x ^= (x >> 23) ^ (x >> 51);

    x
}

fn double_nasam(mut x: u64) -> u64 {
    const M1: u64 = 0x9E6C63D0676A9A99;
    const M2: u64 = 0x9E6D62D06F6A9A9B;

    x ^= x.rotate_right(25) ^ x.rotate_right(47);

    for _ in 0..2 {
        x = x.wrapping_mul(M1);
        x ^= (x >> 23) ^ (x >> 51);

        x = x.wrapping_mul(M2);
        x ^= (x >> 23) ^ (x >> 51);
    }

    x
}

fn avalanche_test<F>(mix: F, samples: u64) -> (f64, f64, f64, f64)
where
    F: Fn(u64) -> u64,
{
    let mut n: u64 = 0;
    let mut sum: u64 = 0;
    let mut sum_sq: u64 = 0;

    for i in 0..samples {
        let input1: u64 = double_nasam(i);

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

fn run_avalanche_test<F>(name: &str, mix: F, samples: u64)
where
    F: Fn(u64) -> u64,
{
    println!("Testing {}:", name);

    let (mean, stddev, mean_z, stddev_z) = avalanche_test(mix, samples);

    println!("Mean Hamming distance: {:9.6} (Z = {:6.2})", mean, mean_z);
    println!("Standard deviation   : {:9.6} (Z = {:6.2})", stddev, stddev_z);
    println!();
}

#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t = 1 << 17)]
    samples: u64,

    #[arg(long, default_value = "all")]
    mixer: String,
}

struct Mixer {
    name: &'static str,
    func: fn(u64) -> u64,
}

fn main() {
    let args = Args::parse();

    println!(
        "Running avalanche tests. Theoretically, the mean should be 32 \
        and the standard deviation should be 4.");
    println!();
    println!("{} samples.", args.samples);
    println!();

    let mixers: &[Mixer] = &[
        Mixer { name: "trivial", func: trivial_mix },
        Mixer { name: "fake_ava", func: fake_ava_mix },
        Mixer { name: "faker_ava", func: faker_ava_mix },
        Mixer { name: "terrible_pi", func: terrible_pi_mix },
        Mixer { name: "lousy_pi", func: lousy_pi_mix },
        Mixer { name: "xorshuffle:4", func: |x| xorshuffle_mix(4, x) },
        Mixer { name: "xorshuffle:5", func: |x| xorshuffle_mix(5, x) },
        Mixer { name: "murmurhash3", func: murmurhash3_mix },
        Mixer { name: "extended_murmurhash3:3", func: |x| extended_murmurhash3_mix(3, x) },
        Mixer { name: "nasam", func: nasam_mix },
    ];

    if args.mixer == "all" {
        for m in mixers {
            run_avalanche_test(m.name, m.func, args.samples);
        }
    } else {
        match mixers.iter().find(|m| m.name == args.mixer) {
            Some(m) => run_avalanche_test(m.name, m.func, args.samples),
            None => panic!("Unknown mixer: {}", args.mixer),
        }
    }
}
