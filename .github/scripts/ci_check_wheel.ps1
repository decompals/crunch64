
param (
    [Parameter(Mandatory = $true)]
    [string]$PYTHON_VERSION,

    [Parameter(Mandatory = $true)]
    [string]$KEY,

    [string]$EXTRA
)

# Equivalent to `set -e`
$ErrorActionPreference = "Stop"

if (Test-Path ".venv") {
    Remove-Item -Recurse -Force ".venv"
}

uv venv -p $PYTHON_VERSION $EXTRA
./.venv/Scripts/Activate.ps1
uv run python --version
uv pip install $(Get-ChildItem -Path .\dist\ -Recurse -Filter "crunch64-*-abi3-*")
uv run python -c "import crunch64; print(crunch64.__version__)"
