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

    use crate::mixers::NASAM;
    use crate::oplistmixer::{CompiledMixer, OpListMixer};

    #[pyclass(name = "OpListMixer")]
    struct PyOpListMixer {
        inner: Box<dyn OpListMixer + Send>,
        compiled: Option<CompiledMixer>,
    }

    #[pymethods]
    impl PyOpListMixer {
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
    }

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        let nasam = PyOpListMixer { inner: Box::new(NASAM), compiled: None };
        m.add("nasam", nasam)?;
        Ok(())
    }
}
