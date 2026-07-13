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

pub mod catalog;

use cranelift::prelude::{
    AbiParam, Block, InstBuilder, FunctionBuilder,
    FunctionBuilderContext, Value,
};
use cranelift_codegen::Context;
use cranelift_codegen::ir::types::I64;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use documented::Documented;
use pyo3::prelude::pymodule;
use std::fmt;
use std::fmt::{Display, Formatter};

use crate::ops::{Operation, OpListBuilder};

#[derive(Clone, Copy)]
pub struct Mixer {
    function: extern "C" fn(u64) -> u64,
}

impl Mixer {
    pub fn mix(&self, x: u64) -> u64 {
        (self.function)(x)
    }
}

pub trait HasDocstring {
    fn docstring(&self) -> String;
}

impl<T: Documented> HasDocstring for T {
    fn docstring(&self) -> String {
        Self::DOCS.to_string()
    }
}

pub trait MixerDef: Display + HasDocstring + Send + Sync {
    fn build(&self, x: &mut OpListBuilder);

    fn operations(&self) -> Vec<Box<dyn Operation>> {
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

struct CustomMixer {
    name: String,
    operations: Vec<Box<dyn Operation>>,
}

impl HasDocstring for CustomMixer {
    fn docstring(&self) -> String {
        "".to_string()
    }
}

impl Display for CustomMixer {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.name)
    }
}

impl MixerDef for CustomMixer {
    fn build(&self, x: &mut OpListBuilder) {
        for op in &self.operations {
            x.push(op)
        }
    }
}

#[pymodule(name = "mixer", module = "xsmtest")]
pub mod py_mixer {
    #[pymodule_export]
    use crate::mixer::catalog::py_catalog;

    use pyo3::prelude::{
        Bound, Py, PyAny, pyclass, pymethods, PyModule, PyRef, PyResult, Python
    };
    use pyo3::types::PyAnyMethods;

    use crate::ops::Operation;
    use crate::ops::py_ops::PyOperation;
    use crate::pybindings::register_submodules;
    use super::{CustomMixer, Mixer, MixerDef};

    #[pyclass(name = "_DocDescriptor")]
    struct DocDescriptor;

    #[pymethods]
    impl DocDescriptor {
        fn __get__<'a>(&self, obj: &Bound<'a, PyAny>, type_: &Bound<'a, PyAny>)
            -> PyResult<Bound<'a, PyAny>>
        {
            if obj.is_none() {
                type_.getattr("_docstring")
            } else {
                obj.getattr("description")
            }
        }
    }

    /// A mixer, composed of several mixer operations
    #[pyclass(name = "MixerDef")]
    pub struct PyMixerDef {
        inner: Box<dyn MixerDef>,
        compiled: Option<Mixer>,
    }

    #[pymethods]
    impl PyMixerDef {
        #[new]
        fn __new__(name: String, py_operations: &Bound<'_, PyAny>)
            -> PyResult<PyMixerDef>
        {
            let mut operations: Vec<Box<dyn Operation>> = Vec::new();

            for item in py_operations.try_iter()? {
                let py_op: PyRef<PyOperation> = item?.extract()?;
                operations.push(py_op.0.box_clone())
            }

            Ok(PyMixerDef::new(Box::new(CustomMixer { name, operations })))
        }

        #[getter]
        fn description(&self) -> String {
            self.inner.docstring()
        }

        fn __call__(&mut self, x: u64) -> u64 {
            self.compile().mix(x)
        }

        fn __str__(&self) -> String {
            self.name()
        }

        fn __repr__<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
            py.eval(c"str.format", None, None)?.call1((
                "MixerDef({!r}, {!r})",
                self.name(),
                self.operations(),
            ))
        }

        #[getter]
        pub fn name(&self) -> String {
            self.inner.to_string()
        }

        #[getter]
        fn operations(&self) -> Vec<PyOperation> {
            self.inner.operations()
                .into_iter()
                .map(|op| PyOperation(op))
                .collect()
        }
    }

    impl PyMixerDef {
        pub fn new(inner: Box<dyn MixerDef>) -> Self {
            Self { inner, compiled: None }
        }

        pub fn compile(&mut self) -> Mixer {
            *self.compiled.get_or_insert_with(|| self.inner.compile())
        }
    }

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        let py = m.py();

        let orig_docstring = m.getattr("MixerDef")?.getattr("__doc__")?;
        m.getattr("MixerDef")?.setattr("_docstring", orig_docstring)?;

        let descriptor = Py::new(py, DocDescriptor { })?;
        m.getattr("MixerDef")?.setattr("__doc__", descriptor)?;

        register_submodules(m)?;

        Ok(())
    }
}

pub use py_mixer::PyMixerDef;
