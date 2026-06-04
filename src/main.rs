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
mod mixertests;
mod oplistmixer;
mod prng;

use clap::Parser;

use crate::mixertests::{
    Avalanche, MixerTest, MixerTestContext,
    Powers, Shift, StrictAvalanche, TestType, Z3,
};
use crate::prng::PRNG;

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

    let test: &dyn MixerTest = match args.test {
        TestType::Avalanche => &Avalanche,
        TestType::Powers => &Powers,
        TestType::Shift => &Shift,
        TestType::StrictAvalanche => &StrictAvalanche,
        TestType::Z3 => &Z3,
    };

    if args.mixer == "all" {
        for m in mixers::MIXERS {
            test.run_test(MixerTestContext {
                prng: prng.get_prng(),
                name: m.name,
                mixer: &*(m.func)(),
                samples: args.samples
            })
        }
    } else {
        match mixers::MIXERS.iter().find(|m| m.name == args.mixer) {
            Some(m) => test.run_test(MixerTestContext {
                prng: prng.get_prng(),
                name: m.name,
                mixer: &*(m.func)(),
                samples: args.samples
            }),
            None => panic!("Unknown mixer: {}", args.mixer),
        }
    }
}
