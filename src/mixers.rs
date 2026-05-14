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
fn deluxe_fake_ava_mix(mut x: u64) -> u64 {
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

// A shift-only mixer created entirely via mutation testing.
fn mutashuffle_mix(mut x: u64) -> u64 {
    x ^= x >> 1;
    x ^= x << 2;
    x ^= x >> 4;
    x ^= x << 8;
    x ^= x >> 16;
    x ^= x << 59;
    x ^= x >> 18;
    x ^= x << 9;
    x ^= x >> 53;
    x ^= x << 63;
    x ^= x >> 29;
    x ^= x << 56;
    x ^= x >> 15;
    x ^= x << 50;

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

// Pelle Evensen's NASAM, but modified to do four multiply rounds instead
// of two. Original algorithm from
// https://mostlymangling.blogspot.com/2020/01/nasam-not-another-strange-acronym-mixer.html
pub fn double_nasam(mut x: u64) -> u64 {
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

pub struct Mixer {
    pub name: &'static str,
    pub func: fn(u64) -> u64,
}

pub const MIXERS: &[Mixer] = &[
    Mixer { name: "trivial", func: trivial_mix },
    Mixer { name: "fake_ava", func: fake_ava_mix },
    Mixer { name: "deluxe_fake_ava", func: deluxe_fake_ava_mix },
    Mixer { name: "terrible_pi", func: terrible_pi_mix },
    Mixer { name: "lousy_pi", func: lousy_pi_mix },
    Mixer { name: "xorshuffle:4", func: |x| xorshuffle_mix(4, x) },
    Mixer { name: "xorshuffle:5", func: |x| xorshuffle_mix(5, x) },
    Mixer { name: "mutashuffle", func: mutashuffle_mix },
    Mixer { name: "murmurhash3", func: murmurhash3_mix },
    Mixer { name: "extended_murmurhash3:3", func: |x| extended_murmurhash3_mix(3, x) },
    Mixer { name: "nasam", func: nasam_mix },
];
