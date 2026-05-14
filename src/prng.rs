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

use crate::mixers::double_nasam as mix;

pu struct PRNG {
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
