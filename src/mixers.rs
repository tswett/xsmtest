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

use documented::Documented;
use pyo3::prelude::pymodule;
use std::fmt::{Display, Formatter, Result};

use crate::mixer::MixerDef;
use crate::ops::OpListBuilder;

// Multiplicative inverse
#[allow(dead_code)]
fn mulinv(x: u64) -> u64 {
    let mut inv = x;

    for _ in 0..5 {
        inv = inv.wrapping_mul(2_u64.wrapping_sub(x.wrapping_mul(inv)));
    }

    inv
}

/// A totally trivial mixing function that literally does nothing.
#[derive(Documented)]
struct Trivial;

impl Display for Trivial {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "trivial")
    }
}

impl MixerDef for Trivial {
    fn build(&self, _x: &mut OpListBuilder) { }
}

/// An atrocious mixing function which gets the mean Hamming distance right, but
/// doesn't actually mix anything.
#[derive(Documented)]
struct FakeAva;

impl Display for FakeAva {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "fake_ava")
    }
}

impl MixerDef for FakeAva {
    fn build(&self, x: &mut OpListBuilder) {
        x.gated_xor(0xffffffff_00000000, 0xffffffff_ffffffff);
    }
}

/// Still an atrocious mixing function which gets both the mean and the standard
/// deviation of the Hamming distance right, but doesn't actually mix anything.
/// This one uses a carefully engineered relationship between the input bit that
/// was flipped in the avalanche test and the number of output bits that flip as
/// a result. Specifically:
///
/// * at 12 input positions, a flip causes 37 output positions to flip,
/// * at 20 input positions, a flip causes 33 output positions to flip,
/// * at 20 input positions, a flip causes 31 output positions to flip, and
/// * at 12 input positions, a flip causes 27 output positions to flip.
///
/// This causes this "mixer" to get the mean and standard deviation correct on
/// the nose (which is _better_ than a real mixer would perform).
#[derive(Documented)]
struct DeluxeFakeAva;

impl Display for DeluxeFakeAva {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "deluxe_fake_ava")
    }
}

impl MixerDef for DeluxeFakeAva {
    fn build(&self, x: &mut OpListBuilder) {
        x.gated_xor(0xffffffff_ffffffff, 0x00000000_ffffffff);
        x.gated_xor(0x00000000_fffff000, 0x00000000_0000000f);
        x.gated_xor(0x000fffff_00000000, 0xf0000000_00000000);
    }
}

// pi * 2^60, rounded to the nearest odd integer
const PI64: u64 = 0x3243f6a8885a308d;

/// This gives us some avalanching, but not much.
#[derive(Documented)]
struct TerriblePi;

impl Display for TerriblePi {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "terrible_pi")
    }
}

impl MixerDef for TerriblePi {
    fn build(&self, x: &mut OpListBuilder) {
        x.multiply(PI64);
    }
}

/// A little bit more avalanching, but not enough.
#[derive(Documented)]
struct LousyPi;

impl Display for LousyPi {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "lousy_pi")
    }
}

impl MixerDef for LousyPi {
    fn build(&self, x: &mut OpListBuilder) {
        x.multiply(PI64);
        x.xorshift_right(32);

        x.multiply(PI64);
    }
}

/// A shift-only mixer created entirely via mutation testing.
#[derive(Documented)]
struct MutaShuffle;

impl Display for MutaShuffle {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "mutashuffle")
    }
}

impl MixerDef for MutaShuffle {
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

/// A terrible mixer that should be easy to automatically analyze.
#[derive(Documented)]
struct EasyNut;

impl Display for EasyNut {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "easynut")
    }
}

impl MixerDef for EasyNut {
    fn build(&self, x: &mut OpListBuilder) {
        x.multiply(19);
        x.xorshift_right(1);

        x.multiply(19);
    }
}

/// A MurmurHash3-like mixer that uses the PI64 constant.
#[derive(Documented)]
struct DecentPi;

impl Display for DecentPi {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "decent_pi")
    }
}

impl MixerDef for DecentPi {
    fn build(&self, x: &mut OpListBuilder) {
        x.xorshift_right(32);

        x.multiply(PI64);
        x.xorshift_right(32);

        x.multiply(PI64);
        x.xorshift_right(32);
    }
}

/// The venerable finalizer from MurmurHash3 (fmix64), taken from
/// https://github.com/aappleby/smhasher/blob/master/src/MurmurHash3.cpp
#[derive(Documented)]
struct MurmurHash3;

impl Display for MurmurHash3 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "murmurhash3")
    }
}

impl MixerDef for MurmurHash3 {
    fn build(&self, x: &mut OpListBuilder) {
        const M1: u64 = 0xff51afd7ed558ccd;
        const M2: u64 = 0xc4ceb9fe1a85ec53;

        x.xorshift_right(33);

        x.multiply(M1);
        x.xorshift_right(33);

        x.multiply(M2);
        x.xorshift_right(33);
    }
}

/// The MurmurHash3 finalizer, but with an arbitrary number of rounds.
#[derive(Documented)]
struct ExtendedMurmurHash3 {
    rounds: u32,
}

impl Display for ExtendedMurmurHash3 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "extended_murmurhash3({})", self.rounds)
    }
}

impl MixerDef for ExtendedMurmurHash3 {
    fn build(&self, x: &mut OpListBuilder) {
        const M1: u64 = 0xff51afd7ed558ccd;
        const M2: u64 = 0xc4ceb9fe1a85ec53;

        x.xorshift_right(33);

        for round in 0..self.rounds {
            x.multiply(if round % 2 == 0 { M1 } else { M2 });
            x.xorshift_right(33);
        }
    }
}

/// David Stafford's Mix13 variant of the finalizer from MurmurHash3, taken from
/// http://zimbry.blogspot.com/2011/09/better-bit-mixing-improving-on.html
///
/// This mixer is used in the SplitMix64 PRNG, and, indeed, it's often called
/// "the SplitMix64 finalizer." This name is doubly incorrect: SplitMix64
/// postdates Mix13 by a couple of years, and although Mix13 is used in
/// SplitMix64, it's not used as the finalizer; SplitMix64 actually uses the same
/// finalizer as MurmurHash3.
///
/// This design is occasionally misattributed to Sebastiano Vigna, who did not
/// design it and has never claimed to have designed it.
#[derive(Documented)]
struct Mix13;

impl Display for Mix13 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "mix13")
    }
}

impl MixerDef for Mix13 {
    fn build(&self, x: &mut OpListBuilder) {
        const M1: u64 = 0xbf58476d1ce4e5b9;
        const M2: u64 = 0x94d049bb133111eb;

        x.xorshift_right(30);

        x.multiply(M1);
        x.xorshift_right(27);

        x.multiply(M2);
        x.xorshift_right(31);
    }
}

/// Pelle Evensen's Moremur, from
/// https://mostlymangling.blogspot.com/2019/12/stronger-better-morer-moremur-better.html
#[derive(Documented)]
struct Moremur;

impl Display for Moremur {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "moremur")
    }
}

impl MixerDef for Moremur {
    fn build(&self, x: &mut OpListBuilder) {
        const M1: u64 = 0x3c79ac492ba7b653;
        const M2: u64 = 0x1c69b3f74ac4ae35;

        x.xorshift_right(27);

        x.multiply(M1);
        x.xorshift_right(33);

        x.multiply(M2);
        x.xorshift_right(27);
    }
}

/// A mixer that does lots of xorrotate.
#[derive(Documented)]
struct RotatoryPi;

impl Display for RotatoryPi {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "rotatory_pi")
    }
}

impl MixerDef for RotatoryPi {
    fn build(&self, x: &mut OpListBuilder) {
        x.xorrotate_right_m(vec!(21, 43));

        x.multiply(PI64);
        x.xorrotate_right_m(vec!(21, 43));

        x.multiply(PI64);
        x.xorrotate_right_m(vec!(21, 43));
    }
}

/// A mixer that does xor(PI64) before and after the other steps.
#[derive(Documented)]
struct PadRotPi;

impl Display for PadRotPi {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "padrot_pi")
    }
}

impl MixerDef for PadRotPi {
    fn build(&self, x: &mut OpListBuilder) {
        x.xor(PI64);

        x.xorrotate_right_m(vec!(21, 43));

        x.multiply(PI64);
        x.xorrotate_right_m(vec!(21, 43));

        x.multiply(PI64);
        x.xorrotate_right_m(vec!(21, 43));

        x.xor(PI64);
    }
}

/// Pelle Evensen's NASAM, from
/// https://mostlymangling.blogspot.com/2020/01/nasam-not-another-strange-acronym-mixer.html
#[derive(Documented)]
struct NASAM;

impl Display for NASAM {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "nasam")
    }
}

impl MixerDef for NASAM {
    fn build(&self, x: &mut OpListBuilder) {
        const M1: u64 = 0x9e6c63d0676a9a99;
        const M2: u64 = 0x9e6d62d06f6a9a9b;

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
    const M1: u64 = 0x9e6c63d0676a9a99;
    const M2: u64 = 0x9e6d62d06f6a9a9b;

    x ^= x.rotate_right(25) ^ x.rotate_right(47);

    for _ in 0..2 {
        x = x.wrapping_mul(M1);
        x ^= (x >> 23) ^ (x >> 51);

        x = x.wrapping_mul(M2);
        x ^= (x >> 23) ^ (x >> 51);
    }

    x
}

/// A mixer modified to do xor(PI64) first
#[derive(Documented)]
struct PreXorPi {
    inner: Box<dyn MixerDef>,
}

impl Display for PreXorPi {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "prexorpi({})", self.inner)
    }
}

impl MixerDef for PreXorPi {
    fn build(&self, x: &mut OpListBuilder) {
        x.xor(PI64);

        self.inner.build(x);
    }
}

pub fn get_mixers() -> Vec<Box<dyn MixerDef>> {
    vec!(
        Box::new(Trivial),
        Box::new(FakeAva),
        Box::new(DeluxeFakeAva),
        Box::new(TerriblePi),
        Box::new(LousyPi),
        Box::new(MutaShuffle),
        Box::new(EasyNut),
        Box::new(DecentPi),
        Box::new(MurmurHash3),
        Box::new(ExtendedMurmurHash3 { rounds: 3 }),
        Box::new(PreXorPi { inner: Box::new(MurmurHash3) }),
        Box::new(Mix13),
        Box::new(Moremur),
        Box::new(RotatoryPi),
        Box::new(PadRotPi),
        Box::new(NASAM),
        Box::new(PreXorPi { inner: Box::new(NASAM) }),
    )
}

#[pymodule(name = "mixers", module = "xsmtest")]
pub mod py_mixers {
    use pyo3::prelude::{Bound, PyModule, PyResult};
    use pyo3::types::PyModuleMethods;

    use crate::mixer::PyMixerDef;
    use super::get_mixers;

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        for mixer in get_mixers() {
            if !mixer.to_string().contains('(') {
                m.add(mixer.to_string(), PyMixerDef::new(mixer))?;
            }
        }

        Ok(())
    }
}
