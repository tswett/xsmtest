// Pelle Evensen's NASAM, but modified to do four multiply rounds instead
// of two. Original algorithm from
// https://mostlymangling.blogspot.com/2020/01/nasam-not-another-strange-acronym-mixer.html

fn mix(mut x: u64) -> u64 {
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

pub struct PRNG {
    // Both the state and the pad are always assumed to be clean, in the sense
    // of not bearing any easily detectable relationship to each other or to
    // any other PRNG's state or pad
    state: u64,
    pad: u64,
}

impl PRNG {
    pub fn from_seed(seed: u64) -> Self {
        PRNG {
            state: mix(seed) ^ mix(seed.wrapping_add(1)),
            pad: mix(seed.wrapping_add(2)) ^ mix(seed.wrapping_add(4))
        }
    }

    pub fn get_number(&mut self) -> u64 {
        let result = mix(self.state) ^ self.pad;
        self.state = self.state.wrapping_add(1);

        result
    }

    pub fn get_prng(&mut self) -> PRNG {
        let state = self.get_number();
        let pad = self.get_number();

        PRNG { state, pad }
    }
}

impl Iterator for PRNG {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.get_number())
    }
}
