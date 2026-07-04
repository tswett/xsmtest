.PHONY: python python-demos

.venv: requirements.txt
	python -m venv .venv
	.venv/bin/pip install -r requirements.txt
	touch .venv

python: .venv
	. .venv/bin/activate && maturin develop && python -i -c 'import xsmtest'

python-demos: .venv
	. .venv/bin/activate && maturin develop && python python/demos.py

pytest: .venv
	. .venv/bin/activate && maturin develop && python -m pytest python/tests
