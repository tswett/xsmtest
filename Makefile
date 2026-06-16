.PHONY: python

.venv:
	python -m venv .venv
	.venv/bin/pip install -r requirements.txt

python: .venv
	. .venv/bin/activate && maturin develop && python -i
