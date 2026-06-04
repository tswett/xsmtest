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
use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::ops::{BitXorAssign, MulAssign, Shr};

pub trait MixerOp {
    #[allow(dead_code)]
    fn eval(&self, x: u64) -> u64;

    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value) -> Value;
}

pub struct MultiplyOp {
    multiplier: u64,
}

impl MultiplyOp {
    pub fn new(multiplier: u64) -> Result<Self, MultiplyOpOperandError> {
        if multiplier % 2 == 1 {
            Ok(MultiplyOp { multiplier })
        } else {
            Err(MultiplyOpOperandError { multiplier })
        }
    }
}

#[derive(Debug)]
pub struct MultiplyOpOperandError {
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

    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value)
        -> Value
    {
        func_builder.ins().imul_imm(input, self.multiplier as i64)
    }
}

pub struct XorshiftRightOp {
    offset: u64,
}

impl XorshiftRightOp {
    pub fn new(offset: u64) -> Result<Self, XorshiftRightOpOperandError> {
        if offset >= 1 && offset <= 64 {
            Ok(XorshiftRightOp { offset })
        } else {
            Err(XorshiftRightOpOperandError { offset })
        }
    }
}

#[derive(Debug)]
pub struct XorshiftRightOpOperandError {
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

    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value)
        -> Value
    {
        let shifted = func_builder.ins().ushr_imm(input, self.offset as i64);
        func_builder.ins().bxor(input, shifted)
    }
}

#[derive(Clone, Copy)]
pub struct OpListBuilder<'a> {
    op_list: &'a RefCell<Vec<Box<dyn MixerOp>>>,
}

pub struct ShiftRightFragment {
    offset: u64
}

impl Shr<u64> for OpListBuilder<'_> {
    type Output = ShiftRightFragment;

    fn shr(self, offset: u64) -> Self::Output {
        ShiftRightFragment { offset }
    }
}

impl BitXorAssign<ShiftRightFragment> for OpListBuilder<'_> {
    fn bitxor_assign(&mut self, fragment: ShiftRightFragment) {
        self.op_list.borrow_mut().push(Box::new(
            XorshiftRightOp::new(fragment.offset).unwrap()
        ));
    }
}

impl MulAssign<u64> for OpListBuilder<'_> {
    fn mul_assign(&mut self, multiplier: u64) {
        self.op_list.borrow_mut().push(Box::new(
            MultiplyOp::new(multiplier).unwrap()
        ));
    }
}

pub struct CompiledMixer {
    function: extern "C" fn(u64) -> u64,
}

impl CompiledMixer {
    pub fn call(&self, x: u64) -> u64 {
        (self.function)(x)
    }
}

pub trait OpListMixer: Sync {
    fn build(&self, x: OpListBuilder);

    fn operations(&self) -> Vec<Box<dyn MixerOp>> {
        let op_list: RefCell<Vec<Box<dyn MixerOp>>> = RefCell::new(vec!());
        self.build(OpListBuilder { op_list: &op_list });
        op_list.take()
    }

    fn compile(&self) -> CompiledMixer {
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

        CompiledMixer {
            function: unsafe { std::mem::transmute(code_ptr) },
        }
    }
}
