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

use crate::oplistmixer::{CompiledMixer, OpListBuilder, OpListMixer};

// Multiplicative inverse
#[allow(dead_code)]
fn mulinv(x: u64) -> u64 {
    let mut inv = x;

    for _ in 0..5 {
        inv = inv.wrapping_mul(2_u64.wrapping_sub(x.wrapping_mul(inv)));
    }

    inv
}

pub trait Mixer: Sync {
    fn mix(&self, x: u64) -> u64;
}

impl Mixer for CompiledMixer {
    fn mix(&self, x: u64) -> u64 {
        self.call(x)
    }
}

// A totally trivial mixing function that literally does nothing.
struct Trivial;
impl OpListMixer for Trivial {
    fn build(&self, _x: &mut OpListBuilder) { }
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
impl OpListMixer for TerriblePi {
    fn build(&self, x: &mut OpListBuilder) {
        x.multiply(PI64);
    }
}

// A little bit more avalanching, but not enough.
struct LousyPi;
impl OpListMixer for LousyPi {
    fn build(&self, x: &mut OpListBuilder) {
        x.multiply(PI64);
        x.xorshift_right(32);

        x.multiply(PI64);
    }
}

struct XorShuffle {
    rounds: u32,
}
impl OpListMixer for XorShuffle {
    fn build(&self, x: &mut OpListBuilder) {
        for _ in 0..self.rounds {
            x.xorshift_right(23);
            x.xorshift_left(17);
            x.xorshift_right(13);
            x.xorshift_left(10);
        }
    }
}

// A shift-only mixer created entirely via mutation testing.
struct MutaShuffle;
impl OpListMixer for MutaShuffle {
    fn build(&self, x: &mut OpListBuilder) {
        x.xorshift_right(1);
        x.xorshift_left(2);
        x.xorshift_right(4);
        x.xorshift_left(8);
        x.xorshift_right(16);
        x.xorshift_left(59);
        x.xorshift_right(18);
        x.xorshift_left(9);
        x.xorshift_right(53);
        x.xorshift_left(63);
        x.xorshift_right(29);
        x.xorshift_left(56);
        x.xorshift_right(15);
        x.xorshift_left(50);
    }
}

// A terrible mixer that should be easy to automatically analyze.
struct EasyNut;
impl OpListMixer for EasyNut {
    fn build(&self, x: &mut OpListBuilder) {
        x.multiply(19);
        x.xorshift_right(1);

        x.multiply(19);
    }
}

// The venerable MurmurHash3 finalizer (fmix64), taken from
// https://github.com/aappleby/smhasher/blob/master/src/MurmurHash3.cpp
struct MurmurHash3;
impl OpListMixer for MurmurHash3 {
    fn build(&self, x: &mut OpListBuilder) {
        const M1: u64 = 0xFF51AFD7ED558CCD;
        const M2: u64 = 0xC4CEB9FE1A85EC53;

        x.xorshift_right(33);

        x.multiply(M1);
        x.xorshift_right(33);

        x.multiply(M2);
        x.xorshift_right(33);
    }
}

struct ExtendedMurmurHash3 {
    rounds: u32,
}
impl OpListMixer for ExtendedMurmurHash3 {
    fn build(&self, x: &mut OpListBuilder) {
        const M1: u64 = 0xFF51AFD7ED558CCD;
        const M2: u64 = 0xC4CEB9FE1A85EC53;

        x.xorshift_right(33);

        for round in 0..self.rounds {
            x.multiply(if round % 2 == 0 { M1 } else { M2 });
            x.xorshift_right(33);
        }
    }
}

// Pelle Evensen's NASAM, from
// https://mostlymangling.blogspot.com/2020/01/nasam-not-another-strange-acronym-mixer.html
struct NASAM;
impl OpListMixer for NASAM {
    fn build(&self, x: &mut OpListBuilder) {
        const M1: u64 = 0x9E6C63D0676A9A99;
        const M2: u64 = 0x9E6D62D06F6A9A9B;

        x.xorrotate_right_m(vec!(25, 47));

        x.multiply(M1);
        x.xorshift_right_m(vec!(23, 51));

        x.multiply(M2);
        x.xorshift_right_m(vec!(23, 51));
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
    pub func: fn() -> Box<dyn Mixer>,
}

pub const MIXERS: &[MixerInfo] = &[
    MixerInfo { name: "trivial", func: || Box::new(Trivial.compile()) },
    MixerInfo { name: "fake_ava", func: || Box::new(FakeAva) },
    MixerInfo { name: "deluxe_fake_ava", func: || Box::new(DeluxeFakeAva) },
    MixerInfo { name: "terrible_pi", func: || Box::new(TerriblePi.compile()) },
    MixerInfo { name: "lousy_pi", func: || Box::new(LousyPi.compile()) },
    MixerInfo {
        name: "xorshuffle:4",
        func: || Box::new(XorShuffle { rounds: 4 }.compile())
    },
    MixerInfo {
        name: "xorshuffle:5",
        func: || Box::new(XorShuffle { rounds: 5 }.compile())
    },
    MixerInfo { name: "mutashuffle", func: || Box::new(MutaShuffle.compile()) },
    MixerInfo { name: "easynut", func: || Box::new(EasyNut.compile()) },
    MixerInfo { name: "murmurhash3", func: || Box::new(MurmurHash3.compile()) },
    MixerInfo {
        name: "extended_murmurhash3:3",
        func: || Box::new(ExtendedMurmurHash3 { rounds: 3 }.compile())
    },
    MixerInfo {
        name: "prexorpi:murmurhash3",
        func: || Box::new(PreXorPi { inner: MurmurHash3.compile() })
    },
    MixerInfo { name: "nasam", func: || Box::new(NASAM.compile()) },
    MixerInfo { name: "double_nasam", func: || Box::new(DoubleNasam) },
    MixerInfo {
        name: "prexorpi:nasam",
        func: || Box::new(PreXorPi { inner: NASAM.compile() })
    },
];
