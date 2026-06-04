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

use std::fmt::{Display, Formatter};

// Multiplicative inverse
#[allow(dead_code)]
fn mulinv(x: u64) -> u64 {
    let mut inv = x;

    for _ in 0..5 {
        inv = inv.wrapping_mul(2_u64.wrapping_sub(x.wrapping_mul(inv)));
    }

    inv
}

trait MixerOp {
    fn eval(&self, x: u64) -> u64;
}

struct MultiplyOp {
    multiplier: u64,
}

impl MultiplyOp {
    fn new(multiplier: u64) -> Result<Self, MultiplyOpOperandError> {
        if multiplier % 2 == 1 {
            Ok(MultiplyOp { multiplier })
        } else {
            Err(MultiplyOpOperandError { multiplier })
        }
    }
}

#[derive(Debug)]
struct MultiplyOpOperandError {
    multiplier: u64,
}

impl Display for MultiplyOpOperandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f,
            "invalid multiplier: expected odd number but {:016x} is even",
            self.multiplier)
    }
}

impl MixerOp for MultiplyOp {
    fn eval(&self, x: u64) -> u64 {
        x.wrapping_mul(self.multiplier)
    }
}

struct XorshiftRightOp {
    offset: u64,
}

impl XorshiftRightOp {
    fn new(offset: u64) -> Result<Self, XorshiftRightOpOperandError> {
        if offset >= 1 && offset <= 64 {
            Ok(XorshiftRightOp { offset })
        } else {
            Err(XorshiftRightOpOperandError { offset })
        }
    }
}

#[derive(Debug)]
struct XorshiftRightOpOperandError {
    offset: u64,
}

impl Display for XorshiftRightOpOperandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f,
            "invalid offset: expected number in range 1 to 64 but {} is even",
            self.offset)
    }
}

impl MixerOp for XorshiftRightOp {
    fn eval(&self, x: u64) -> u64 {
        x ^ (x >> self.offset)
    }
}

pub trait Mixer: Sync {
    fn mix(&self, x: u64) -> u64;
}

trait OpListMixer: Sync {
    fn operations(&self) -> Vec<Box<dyn MixerOp>>;
}

impl<M: OpListMixer> Mixer for M {
    fn mix(&self, mut x: u64) -> u64 {
        for op in self.operations().iter() {
            x = op.eval(x);
        }

        x
    }
}

// A totally trivial mixing function that literally does nothing.
struct Trivial;
impl Mixer for Trivial {
    fn mix(&self, x: u64) -> u64 {
        x
    }
}

// An atrocious mixing function which gets the mean Hamming distance right, but
// doesn't actually mix anything.
struct FakeAva;
impl Mixer for FakeAva {
    fn mix(&self, x: u64) -> u64 {
        if (x >> 32).count_ones() % 2 == 1 {
            !x
        } else {
            x
        }
    }
}

// Still an atrocious mixing function which gets both the mean and the standard
// deviation of the Hamming distance right, but doesn't actually mix anything.
// This one uses a carefully engineered relationship between the input bit that
// was flipped in the avalanche test and the number of output bits that flip as
// a result. Specifically:
//
// * at 12 input positions, a flip causes 37 output positions to flip,
// * at 20 input positions, a flip causes 33 output positions to flip,
// * at 20 input positions, a flip causes 31 output positions to flip, and
// * at 12 input positions, a flip causes 27 output positions to flip.
//
// This causes this "mixer" to get the mean and standard deviation correct on
// the nose (which is _better_ than a real mixer would perform).
struct DeluxeFakeAva;
impl Mixer for DeluxeFakeAva {
    fn mix(&self, mut x: u64) -> u64 {
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
}

// pi * 2^60, rounded to the nearest odd integer
const PI64: u64 = 0x3243F6A8885A308D;

// This gives us some avalanching, but not much.
struct TerriblePi;
impl Mixer for TerriblePi {
    fn mix(&self, x: u64) -> u64 {
        x.wrapping_mul(PI64)
    }
}

// A little bit more avalanching, but not enough.
struct LousyPi;
impl Mixer for LousyPi {
    fn mix(&self, mut x: u64) -> u64 {
        x = x.wrapping_mul(PI64);
        x ^= x >> 32;

        x = x.wrapping_mul(PI64);

        x
    }
}

struct XorShuffle {
    rounds: u32,
}
impl Mixer for XorShuffle {
    fn mix(&self, mut x: u64) -> u64 {
        for _ in 0..self.rounds {
            x ^= x >> 23;
            x ^= x << 17;
            x ^= x >> 13;
            x ^= x << 10;
        }

        x
    }
}

// A shift-only mixer created entirely via mutation testing.
struct MutaShuffle;
impl Mixer for MutaShuffle {
    fn mix(&self, mut x: u64) -> u64 {
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
}

// A terrible mixer that should be easy to automatically analyze.
struct EasyNut;
impl Mixer for EasyNut {
    fn mix(&self, mut x: u64) -> u64 {
        x = x.wrapping_mul(19);
        x ^= x >> 1;
        x = x.wrapping_mul(19);

        x
    }
}

// The venerable MurmurHash3 finalizer (fmix64), taken from
// https://github.com/aappleby/smhasher/blob/master/src/MurmurHash3.cpp
struct MurmurHash3;
impl OpListMixer for MurmurHash3 {
    fn operations(&self) -> Vec<Box<dyn MixerOp>> {
        const M1: u64 = 0xFF51AFD7ED558CCD;
        const M2: u64 = 0xC4CEB9FE1A85EC53;

        vec!(
            Box::new(XorshiftRightOp::new(33).unwrap()),
            Box::new(MultiplyOp::new(M1).unwrap()),
            Box::new(XorshiftRightOp::new(33).unwrap()),
            Box::new(MultiplyOp::new(M2).unwrap()),
            Box::new(XorshiftRightOp::new(33).unwrap()),
        )
    }
}

struct ExtendedMurmurHash3 {
    rounds: u32,
}
impl Mixer for ExtendedMurmurHash3 {
    fn mix(&self, mut x: u64) -> u64 {
        const M1: u64 = 0xFF51AFD7ED558CCD;
        const M2: u64 = 0xC4CEB9FE1A85EC53;

        x ^= x >> 33;

        for round in 0..self.rounds {
            x = x.wrapping_mul(if round % 2 == 0 { M1 } else { M2 });
            x ^= x >> 33;
        }

        x
    }
}

// Pelle Evensen's NASAM, from
// https://mostlymangling.blogspot.com/2020/01/nasam-not-another-strange-acronym-mixer.html
struct NASAM;
impl Mixer for NASAM {
    fn mix(&self, mut x: u64) -> u64 {
        const M1: u64 = 0x9E6C63D0676A9A99;
        const M2: u64 = 0x9E6D62D06F6A9A9B;

        x ^= x.rotate_right(25) ^ x.rotate_right(47);

        x = x.wrapping_mul(M1);
        x ^= (x >> 23) ^ (x >> 51);

        x = x.wrapping_mul(M2);
        x ^= (x >> 23) ^ (x >> 51);

        x
    }
}

// NASAM, modified to do four rounds instead of two
// 
// Exposed as a function so that the PRNG module can use it easily.
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

struct DoubleNasam;
impl Mixer for DoubleNasam {
    fn mix(&self, x: u64) -> u64 {
        double_nasam(x)
    }
}

struct PreXorPi<M: Mixer> {
    inner: M,
}
impl<M: Mixer> Mixer for PreXorPi<M> {
    fn mix(&self, x: u64) -> u64 {
        self.inner.mix(x ^ PI64)
    }
}

pub struct MixerInfo<'a> {
    pub name: &'a str,
    pub func: &'a dyn Mixer,
}

pub const MIXERS: &[MixerInfo] = &[
    MixerInfo { name: "trivial", func: &Trivial { } },
    // MixerInfo {
    //     name: "xorpi",
    //     func: &PreXorPi { inner: Trivial { } }
    // },
    MixerInfo { name: "fake_ava", func: &FakeAva { } },
    MixerInfo { name: "deluxe_fake_ava", func: &DeluxeFakeAva { } },
    MixerInfo { name: "terrible_pi", func: &TerriblePi { } },
    // MixerInfo {
    //     name: "prexorpi:terrible_pi",
    //     func: &PreXorPi { inner: TerriblePi { } }
    // },
    MixerInfo { name: "lousy_pi", func: &LousyPi { } },
    MixerInfo { name: "xorshuffle:4", func: &XorShuffle { rounds: 4 } },
    MixerInfo { name: "xorshuffle:5", func: &XorShuffle { rounds: 5 } },
    MixerInfo { name: "mutashuffle", func: &MutaShuffle },
    MixerInfo { name: "easynut", func: &EasyNut { } },
    MixerInfo { name: "murmurhash3", func: &MurmurHash3 },
    MixerInfo { name: "extended_murmurhash3:3", func: &ExtendedMurmurHash3 { rounds: 3 } },
    MixerInfo {
        name: "prexorpi:murmurhash3",
        func: &PreXorPi { inner: MurmurHash3 { } }
    },
    MixerInfo { name: "nasam", func: &NASAM { } },
    MixerInfo { name: "double_nasam", func: &DoubleNasam { } },
    MixerInfo {
        name: "prexorpi:nasam",
        func: &PreXorPi { inner: NASAM { } }
    },
];
