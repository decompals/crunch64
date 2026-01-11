
PYTHON_VERSION=$1
KEY=$2

set -e

rm -rf .venv
uv venv -p $PYTHON_VERSION --managed-python
source .venv/bin/activate
uv run python --version
uv pip install $(find ./dist/ -name "crunch64-*-$KEY*")
uv run python -c "import crunch64; print(crunch64.__version__)"
