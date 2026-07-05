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

use cranelift::prelude::{FunctionBuilder, InstBuilder, Value};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::{PyErr, pymodule};
use std::fmt;
use std::fmt::{Display, Formatter};

pub trait Operation: Display + Send + Sync {
    fn box_clone(&self) -> Box<dyn Operation>;
    fn eval(&self, x: u64) -> u64;
    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value)
        -> Value;
}

#[derive(Debug)]
pub enum ParameterError {
    MultiplierError { multiplier: u64 },
    OffsetError { offset: i32 },
    RotateCountError { count: usize },
    GatePadError { gate: u64, pad: u64 },
}

impl Display for ParameterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            ParameterError::MultiplierError { multiplier } => write!(f,
                "invalid multiplier: expected odd number but 0x{:016x} is even",
                multiplier),
            ParameterError::OffsetError { offset } => write!(f,
                "invalid offset: expected number in range 1 to 64 but {} is outside that range",
                offset),
            ParameterError::RotateCountError { count } => write!(f,
                "invalid number of offsets: expected an even number of offsets but there are {} here",
                count),
            ParameterError::GatePadError { gate, pad } => write!(f,
                "invalid gate and pad: the gate (0x{:016x}) and the pad (0x{:016x}) should have an even number of bits in common but they actually have {}",
                gate,
                pad,
                (gate & pad).count_ones()),
        }
    }
}

impl From<ParameterError> for PyErr {
    fn from(e: ParameterError) -> PyErr {
        PyValueError::new_err(e.to_string())
    }
}

#[derive(Clone, Copy)]
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "multiply(0x{:016x})", self.multiplier)
    }
}

impl Operation for MultiplyOp {
    fn box_clone(&self) -> Box<dyn Operation> {
        Box::new(*self)
    }

    fn eval(&self, x: u64) -> u64 {
        x.wrapping_mul(self.multiplier)
    }

    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value)
        -> Value
    {
        func_builder.ins().imul_imm(input, self.multiplier as i64)
    }
}

#[derive(Clone)]
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "xorshift_right(")?;
        for (i, offset) in self.offsets.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}", offset)?;
        }
        write!(f, ")")
    }
}

impl Operation for XorshiftRightOp {
    fn box_clone(&self) -> Box<dyn Operation> {
        Box::new(self.clone())
    }

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

#[derive(Clone)]
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "xorshift_left(")?;
        for (i, offset) in self.offsets.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}", offset)?;
        }
        write!(f, ")")
    }
}

impl Operation for XorshiftLeftOp {
    fn box_clone(&self) -> Box<dyn Operation> {
        Box::new(self.clone())
    }

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

#[derive(Clone)]
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "xorrotate_right(")?;
        for (i, offset) in self.offsets.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}", offset)?;
        }
        write!(f, ")")
    }
}

impl Operation for XorrotateRightOp {
    fn box_clone(&self) -> Box<dyn Operation> {
        Box::new(self.clone())
    }

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

#[derive(Clone, Copy)]
pub struct XorOp {
    pad: u64,
}

impl XorOp {
    pub fn new(pad: u64) -> Self {
        XorOp { pad }
    }
}

impl Display for XorOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "xor(0x{:016x})", self.pad)
    }
}

impl Operation for XorOp {
    fn box_clone(&self) -> Box<dyn Operation> {
        Box::new(*self)
    }

    fn eval(&self, x: u64) -> u64 {
        x ^ self.pad
    }

    fn compile(&self, func_builder: &mut FunctionBuilder, input: Value)
        -> Value
    {
        func_builder.ins().bxor_imm(input, self.pad as i64)
    }
}

#[derive(Clone, Copy)]
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "gated_xor(0x{:016x}, 0x{:016x})", self.gate, self.pad)
    }
}

impl Operation for GatedXorOp {
    fn box_clone(&self) -> Box<dyn Operation> {
        Box::new(*self)
    }

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
    pub op_list: Vec<Box<dyn Operation>>,
}

impl OpListBuilder {
    pub fn push(&mut self, op: &Box<dyn Operation>) {
        self.op_list.push(op.box_clone());
    }

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

#[pymodule(name = "ops", module = "xsmtest")]
pub mod py_ops {
    use pyo3::prelude::{pyclass, pyfunction, pymethods, PyResult};

    use super::{
        GatedXorOp, MultiplyOp, Operation, XorOp,
        XorrotateRightOp, XorshiftLeftOp, XorshiftRightOp,
    };

    #[pyclass(name = "Operation")]
    pub struct PyOperation(pub Box<dyn Operation>);

    #[pymethods]
    impl PyOperation {
        fn __call__(&self, x: u64) -> u64 {
            self.0.eval(x)
        }

        fn __repr__(&self) -> String {
            self.0.to_string()
        }
    }

    #[pyfunction]
    fn multiply(multiplier: u64) -> PyResult<PyOperation> {
        Ok(PyOperation(Box::new(MultiplyOp::new(multiplier)?)))
    }

    #[pyfunction]
    #[pyo3(signature = (*offsets))]
    fn xorshift_right(offsets: Vec<i32>) -> PyResult<PyOperation> {
        Ok(PyOperation(Box::new(XorshiftRightOp::new(offsets)?)))
    }

    #[pyfunction]
    #[pyo3(signature = (*offsets))]
    fn xorshift_left(offsets: Vec<i32>) -> PyResult<PyOperation> {
        Ok(PyOperation(Box::new(XorshiftLeftOp::new(offsets)?)))
    }

    #[pyfunction]
    #[pyo3(signature = (*offsets))]
    fn xorrotate_right(offsets: Vec<i32>) -> PyResult<PyOperation> {
        Ok(PyOperation(Box::new(XorrotateRightOp::new(offsets)?)))
    }

    #[pyfunction]
    fn xor(pad: u64) -> PyResult<PyOperation> {
        Ok(PyOperation(Box::new(XorOp::new(pad))))
    }

    #[pyfunction]
    fn gated_xor(gate: u64, pad: u64) -> PyResult<PyOperation> {
        Ok(PyOperation(Box::new(GatedXorOp::new(gate, pad)?)))
    }
}
