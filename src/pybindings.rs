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

use pyo3::prelude::pymodule;

#[pymodule]
mod xsmtest {
    use pyo3::prelude::*;

    use crate::oplistmixer::{CompiledMixer, MixerDef, MixerOp};

    #[pyclass(name = "MixerDef")]
    struct PyMixerDef {
        inner: Box<dyn MixerDef>,
        compiled: Option<CompiledMixer>,
    }

    #[pymethods]
    impl PyMixerDef {
        fn __call__(&mut self, x: u64) -> u64 {
            match self.compiled {
                Some(f) => f.call(x),
                None => {
                    let f: CompiledMixer = self.inner.compile();
                    self.compiled = Some(f);
                    f.call(x)
                }
            }
        }

        #[getter]
        fn operations(&self) -> Vec<PyMixerOp> {
            self.inner.operations()
                .into_iter()
                .map(|op| PyMixerOp { inner: op })
                .collect()
        }
    }

    impl PyMixerDef {
        fn new(inner: impl MixerDef + 'static) -> Self {
            Self { inner: Box::new(inner), compiled: None }
        }
    }

    #[pyclass(name = "MixerOp")]
    struct PyMixerOp {
        inner: Box<dyn MixerOp>,
    }

    #[pymethods]
    impl PyMixerOp {
        fn __call__(&self, x: u64) -> u64 {
            self.inner.eval(x)
        }

        fn __repr__(&self) -> String {
            self.inner.to_string()
        }
    }

    #[pymodule]
    mod mixers {
        use pyo3::prelude::*;

        use crate::mixers::{MurmurHash3, NASAM, Trivial};
        use crate::pybindings::xsmtest::PyMixerDef;

        #[pymodule_init]
        fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
            m.add("trivial", PyMixerDef::new(Trivial))?;
            m.add("murmurhash3", PyMixerDef::new(MurmurHash3))?;
            m.add("nasam", PyMixerDef::new(NASAM))?;
            Ok(())
        }
    }
}
