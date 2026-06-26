.PHONY: python python-demos

.venv:
	python -m venv .venv
	.venv/bin/pip install -r requirements.txt

python: .venv
	. .venv/bin/activate && maturin develop && python -i -c 'import xsmtest'

python-demos: .venv
	. .venv/bin/activate && maturin develop && python python/demos.py
