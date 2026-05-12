// A totally trivial mixing function that literally does nothing.
fn trivial_mix(x: u64) -> u64 {
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

pub fn avalanche_test<F>(mix: F) -> (f64, f64)
where
    F: Fn(u64) -> u64,
{
    let mut n: u32 = 0;
    let mut sum: u32 = 0;
    let mut sum_sq: u32 = 0;

    for i in 0..(1 << 14) {
        let input1: u64 = double_nasam(i);

        let output1 = mix(input1);

        for bit in 0..64 {
            let input2 = input1 ^ (1u64 << bit);
            let output2 = mix(input2);

            let distance = (output1 ^ output2).count_ones() as u32;

            n += 1;
            sum += distance;
            sum_sq += distance * distance;
        }
    }

    let mean = (sum as f64) / (n as f64);
    let variance = (sum_sq as f64) / (n as f64) - mean * mean;
    let stddev = variance.sqrt();

    (mean, stddev)
}

pub fn run_avalanche_test<F>(name: &str, mix: F)
where
    F: Fn(u64) -> u64,
{
    println!("Testing {}...", name);

    let (mean, stddev) = avalanche_test(mix);

    println!("Mean Hamming distance : {:.6}", mean);
    println!("Std dev               : {:.6}", stddev);
    println!();
}

fn main() {
    println!(
        "Running avalanche tests. Theoretically, the mean should be 32 \
        and the standard deviation should be 4.");
    println!();

    run_avalanche_test("trivial_mix", trivial_mix);
    run_avalanche_test("nasam_mix", nasam_mix);
}
