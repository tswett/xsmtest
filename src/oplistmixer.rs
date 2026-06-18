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

use cranelift::prelude::{
    AbiParam, Block, InstBuilder, FunctionBuilder,
    FunctionBuilderContext, Value,
};
use cranelift_codegen::Context;
use cranelift_codegen::ir::types::I64;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use std::fmt::{Display, Error, Formatter};

pub trait MixerOp: Display + Send + Sync {
    fn eval(&self, x: u64) -> u64;

    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value) -> Value;
}

#[derive(Debug)]
pub enum ParameterError {
    MultiplierError { multiplier: u64 },
    OffsetError { offset: i32 },
    RotateCountError { count: usize },
    GatePadError { gate: u64, pad: u64 },
}

impl Display for ParameterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            ParameterError::MultiplierError { multiplier } => write!(f,
                "invalid multiplier: expected odd number but {:016x} is even",
                multiplier),
            ParameterError::OffsetError { offset } => write!(f,
                "invalid offset: expected number in range 1 to 64 but {} is outside that range",
                offset),
            ParameterError::RotateCountError { count } => write!(f,
                "invalid number of offsets: expected an even number of offsets but there are {} here",
                count),
            ParameterError::GatePadError { gate, pad } => write!(f,
                "invalid gate and pad: the gate ({:016x}) and the pad ({:016x}) should have an even number of bits in common but they actually have {}",
                gate,
                pad,
                (gate & pad).count_ones()),
        }
    }
}

pub struct MultiplyOp {
    multiplier: u64,
}

impl MultiplyOp {
    pub fn new(multiplier: u64) -> Result<Self, ParameterError> {
        if multiplier % 2 == 1 {
            Ok(MultiplyOp { multiplier })
        } else {
            Err(ParameterError::MultiplierError { multiplier })
        }
    }
}

impl Display for MultiplyOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "multiply(0x{:016x})", self.multiplier)
    }
}

impl MixerOp for MultiplyOp {
    fn eval(&self, x: u64) -> u64 {
        x.wrapping_mul(self.multiplier)
    }

    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value)
        -> Value
    {
        func_builder.ins().imul_imm(input, self.multiplier as i64)
    }
}

pub struct XorshiftRightOp {
    offsets: Vec<i32>,
}

impl XorshiftRightOp {
    pub fn new(offsets: Vec<i32>) -> Result<Self, ParameterError> {
        for &offset in &offsets {
            if !(offset >= 1 && offset <= 64) {
                return Err(ParameterError::OffsetError { offset })
            }
        }

        Ok(XorshiftRightOp { offsets })
    }

    pub fn new_single(offset: i32) -> Result<Self, ParameterError> {
        Self::new(vec!(offset))
    }
}

impl Display for XorshiftRightOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "xorshift_right(")?;
        for (i, offset) in self.offsets.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}", offset)?;
        }
        write!(f, ")")
    }
}

impl MixerOp for XorshiftRightOp {
    fn eval(&self, x: u64) -> u64 {
        let mut result = x;
        for offset in &self.offsets {
            result ^= x >> offset;
        }
        result
    }

    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value)
        -> Value
    {
        let mut result = input;
        for offset in &self.offsets {
            let shifted = func_builder.ins().ushr_imm(input, *offset as i64);
            result = func_builder.ins().bxor(result, shifted);
        }
        result
    }
}

pub struct XorshiftLeftOp {
    offsets: Vec<i32>,
}

impl XorshiftLeftOp {
    pub fn new(offsets: Vec<i32>) -> Result<Self, ParameterError> {
        for &offset in &offsets {
            if !(offset >= 1 && offset <= 64) {
                return Err(ParameterError::OffsetError { offset })
            }
        }

        Ok(XorshiftLeftOp { offsets })
    }

    pub fn new_single(offset: i32) -> Result<Self, ParameterError> {
        Self::new(vec!(offset))
    }
}

impl Display for XorshiftLeftOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "xorshift_left(")?;
        for (i, offset) in self.offsets.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}", offset)?;
        }
        write!(f, ")")
    }
}

impl MixerOp for XorshiftLeftOp {
    fn eval(&self, x: u64) -> u64 {
        let mut result = x;
        for offset in &self.offsets {
            result ^= x << offset;
        }
        result
    }

    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value)
        -> Value
    {
        let mut result = input;
        for offset in &self.offsets {
            let shifted = func_builder.ins().ishl_imm(input, *offset as i64);
            result = func_builder.ins().bxor(result, shifted);
        }
        result
    }
}

pub struct XorrotateRightOp {
    offsets: Vec<i32>,
}

impl XorrotateRightOp {
    pub fn new(offsets: Vec<i32>) -> Result<Self, ParameterError> {
        if offsets.len() % 2 != 0 {
            return Err(ParameterError::RotateCountError { count: offsets.len() })
        }

        for &offset in &offsets {
            if !(offset >= 1 && offset <= 64) {
                return Err(ParameterError::OffsetError { offset })
            }
        }

        Ok(XorrotateRightOp { offsets })
    }
}

impl Display for XorrotateRightOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "xorrotate_right(")?;
        for (i, offset) in self.offsets.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}", offset)?;
        }
        write!(f, ")")
    }
}

impl MixerOp for XorrotateRightOp {
    fn eval(&self, x: u64) -> u64 {
        let mut result = x;
        for offset in &self.offsets {
            result ^= x.rotate_right(*offset as u32);
        }
        result
    }

    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value)
        -> Value
    {
        let mut result = input;
        for offset in &self.offsets {
            let shifted = func_builder.ins().rotr_imm(input, *offset as i64);
            result = func_builder.ins().bxor(result, shifted);
        }
        result
    }
}

pub struct XorOp {
    pad: u64,
}

impl XorOp {
    pub fn new(pad: u64) -> Self {
        XorOp { pad }
    }
}

impl Display for XorOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "xor(0x{:016x})", self.pad)
    }
}

impl MixerOp for XorOp {
    fn eval(&self, x: u64) -> u64 {
        x ^ self.pad
    }

    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value)
        -> Value
    {
        func_builder.ins().bxor_imm(input, self.pad as i64)
    }
}

pub struct GatedXorOp {
    gate: u64,
    pad: u64,
}

impl GatedXorOp {
    pub fn new(gate: u64, pad: u64) -> Result<Self, ParameterError> {
        if (gate & pad).count_ones() % 2 != 0 {
            return Err(ParameterError::GatePadError { gate, pad })
        }

        Ok(GatedXorOp { gate, pad })
    }
}

impl Display for GatedXorOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "gated_xor(0x{:016x}, 0x{:016x})", self.gate, self.pad)
    }
}

impl MixerOp for GatedXorOp {
    fn eval(&self, x: u64) -> u64 {
        if (x & self.gate).count_ones() % 2 == 1 { x ^ self.pad } else { x }
    }

    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value)
        -> Value
    {
        let masked = func_builder.ins().band_imm(input, self.gate as i64);
        let popcount = func_builder.ins().popcnt(masked);
        let parity = func_builder.ins().band_imm(popcount, 1);

        let xored = func_builder.ins().bxor_imm(input, self.pad as i64);

        func_builder.ins().select(parity, xored, input)
    }
}

#[derive(Default)]
pub struct OpListBuilder {
    op_list: Vec<Box<dyn MixerOp>>,
}

impl OpListBuilder {
    pub fn xorshift_right(&mut self, offset: i32) {
        self.op_list.push(Box::new(
            XorshiftRightOp::new_single(offset).unwrap()));
    }

    pub fn xorshift_left(&mut self, offset: i32) {
        self.op_list.push(Box::new(
            XorshiftLeftOp::new_single(offset).unwrap()));
    }

    pub fn xorshift_right_m(&mut self, offsets: Vec<i32>) {
        self.op_list.push(Box::new(
            XorshiftRightOp::new(offsets).unwrap()));
    }

    #[allow(dead_code)]
    pub fn xorshift_left_m(&mut self, offsets: Vec<i32>) {
        self.op_list.push(Box::new(
            XorshiftLeftOp::new(offsets).unwrap()));
    }

    pub fn xorrotate_right_m(&mut self, offsets: Vec<i32>) {
        self.op_list.push(Box::new(
            XorrotateRightOp::new(offsets).unwrap()));
    }

    pub fn xor(&mut self, pad: u64) {
        self.op_list.push(Box::new(
            XorOp::new(pad)));
    }

    pub fn multiply(&mut self, multiplier: u64) {
        self.op_list.push(Box::new(
            MultiplyOp::new(multiplier).unwrap()));
    }

    pub fn gated_xor(&mut self, gate: u64, pad: u64) {
        self.op_list.push(Box::new(
            GatedXorOp::new(gate, pad).unwrap()));
    }
}

#[derive(Clone, Copy)]
pub struct Mixer {
    function: extern "C" fn(u64) -> u64,
}

impl Mixer {
    pub fn mix(&self, x: u64) -> u64 {
        (self.function)(x)
    }
}

pub trait MixerDef: Send + Sync {
    fn build(&self, x: &mut OpListBuilder);

    fn operations(&self) -> Vec<Box<dyn MixerOp>> {
        let mut builder = OpListBuilder::default();
        self.build(&mut builder);
        builder.op_list
    }

    fn compile(&self) -> Mixer {
        let builder: JITBuilder =
            JITBuilder::new(cranelift_module::default_libcall_names()).unwrap();
        let mut module = JITModule::new(builder);

        let mut ctx: Context = module.make_context();

        ctx.func.signature.params.push(AbiParam::new(I64));
        ctx.func.signature.returns.push(AbiParam::new(I64));

        let func_id: FuncId = module
            .declare_function("mix", Linkage::Export, &ctx.func.signature)
            .unwrap();

        let mut func_ctx = FunctionBuilderContext::new();
        let mut func_builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);

        let block: Block = func_builder.create_block();
        func_builder.append_block_params_for_function_params(block);
        func_builder.switch_to_block(block);
        func_builder.seal_block(block);

        let mut x: Value = func_builder.block_params(block)[0];

        for op in self.operations() {
            x = op.compile(&mut func_builder, x);
        }

        func_builder.ins().return_(&[x]);

        func_builder.finalize();

        module.define_function(func_id, &mut ctx).unwrap();
        module.clear_context(&mut ctx);
        module.finalize_definitions().unwrap();

        let code_ptr: *const u8 = module.get_finalized_function(func_id);

        Mixer {
            function: unsafe { std::mem::transmute(code_ptr) },
        }
    }
}
