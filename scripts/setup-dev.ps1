# Bootstrap a native Windows dev environment for pypdfcraft.
# Usage:  powershell -ExecutionPolicy Bypass -File scripts\setup-dev.ps1

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

Write-Host "pypdfcraft root: $Root"

$Pdfcraft = Join-Path (Split-Path $Root -Parent) "pdfcraft"
if (-not (Test-Path "$Pdfcraft\Cargo.toml")) {
    Write-Warning "Expected pdfcraft at $Pdfcraft"
    Write-Warning "Create a junction:  mklink /J `"$Pdfcraft`" M:\path\to\pdfcraft"
}

function Test-Command($name) {
    $null -ne (Get-Command $name -ErrorAction SilentlyContinue)
}

if (-not (Test-Command cargo)) {
    Write-Host "Installing Rust via winget (rustup)..."
    winget install Rustlang.Rustup --accept-package-agreements --accept-source-agreements --disable-interactivity
    $env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
}

if (Test-Command rustup) {
    rustup default stable
    rustup update stable
    rustc --version
} elseif (-not (Test-Command cargo)) {
    Write-Error "cargo not on PATH. Restart the terminal after rustup install, then re-run this script."
}

if (-not (Test-Path ".venv")) {
    python -m venv .venv
}
& .\.venv\Scripts\python.exe -m pip install -U pip maturin pytest

Write-Host @"

Next steps:
  .\.venv\Scripts\Activate.ps1
  maturin develop --release
  pytest

Docker (Linux wheels, Docker Desktop running):
  cd $(Split-Path $Root -Parent)
  docker compose -f pypdfcraft\docker\compose.yml run --rm test

See docs\development.md for MSVC Build Tools and troubleshooting.
"@
