# Local development

pypdfcraft is a **PyO3** extension that links against a **sibling checkout** of [pdfcraft](https://github.com/storytold/pdfcraft). You need Rust, Python, and the two repos on disk (or in Docker; see below).

## Directory layout

```text
D:\code\                    (or any parent)
  pdfcraft\                 # storytold/pdfcraft (clone or junction)
  pypdfcraft\               # this repo
```

If pdfcraft lives elsewhere on Windows, create a junction once (elevated `cmd`):

```bat
mklink /J D:\code\pdfcraft M:\GITHUB\storytold\pdfcraft
```

Cargo path deps in [Cargo.toml](../Cargo.toml) use `../pdfcraft/...` and `../pdfcraft/vendor/...` for patched crates.

## Rust toolchain (Windows)

**MSRV:** Rust **1.94+** (`rust-toolchain.toml`; pdfcraft's `rten` 0.26 requires it).

### Option A — winget (recommended)

```powershell
winget install Rustlang.Rustup --accept-package-agreements --accept-source-agreements
```

Close and reopen the terminal (or log out) so `PATH` includes `%USERPROFILE%\.cargo\bin`.

```powershell
rustup default stable
rustup update stable
rustc --version    # must be >= 1.94
cargo --version
```

### Option B — rustup installer

Download [rustup-init](https://win.rustup.rs/) and run it. Choose the default **stable** toolchain.

### MSVC linkers

On Windows, Rust uses the **MSVC** ABI. Install **Visual Studio Build Tools** (or full VS) with the **“Desktop development with C++”** workload so `link.exe` is available. Without it, `cargo build` fails with linker errors.

```powershell
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

(Adjust year/edition to what you already use.)

## Python environment

CPython **3.10+** (3.12 is a good default). ABI3 wheels target 3.10+.

```powershell
cd D:\code\pypdfcraft
python -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install -U pip maturin pytest
```

## Build and test (native)

First compile is slow (full pdfcraft dependency graph).

```powershell
cd D:\code\pypdfcraft
.\.venv\Scripts\Activate.ps1
maturin develop --release
python -c "import pypdfcraft as p; print(p.__version__)"
pytest
cargo test --release
```

Iterative Rust edits: `maturin develop --release` again, or `maturin develop` for faster debug builds.

### Useful commands

| Task | Command |
|------|---------|
| Release extension in venv | `maturin develop --release` |
| Wheel in `dist/` | `maturin build --release --out dist` |
| Rust unit tests only | `cargo test` |
| Clean Rust artifacts | `cargo clean` |

## Docker (Linux build / CI parity)

Docker avoids installing MSVC + Rust on the host and matches **manylinux** CI. **Docker Desktop must be running** (Linux engine).

### One-shot wheel build

From the **pypdfcraft** repo (compose paths assume `D:\code\pdfcraft` beside `D:\code\pypdfcraft`):

```powershell
cd D:\code\pypdfcraft
docker compose -f docker\compose.yml build wheel
docker compose -f docker\compose.yml run --rm wheel
```

Wheels land in `pypdfcraft\dist\`.

Environment overrides:

| Variable | Default | Meaning |
|----------|---------|---------|
| `PDFCRAFT_ROOT` | `../pdfcraft` (host path relative to compose file) | pdfcraft checkout |
| `PYPDFCRAFT_ROOT` | `..` | pypdfcraft repo (compose parent) |

On Windows, set absolute paths if defaults fail:

```powershell
$env:PDFCRAFT_ROOT = "D:\code\pdfcraft"
$env:PYPDFCRAFT_ROOT = "D:\code\pypdfcraft"
cd D:\code\pypdfcraft
docker compose -f docker\compose.yml run --rm dev
```

### Interactive dev shell in container

```powershell
docker compose -f docker\compose.yml run --rm dev
# inside container:
maturin develop --release
pytest
```

Sources are bind-mounted; rebuilds use your working tree. Compose masks host `.venv` and `target/` with container volumes so a Windows venv and MSVC artifacts do not break Linux builds.

### Full CI-style test in Docker

```powershell
cd D:\code\pypdfcraft
docker compose -f docker\compose.yml build test
docker compose -f docker\compose.yml run --rm test
```

Runs `maturin develop --release` and `pytest` inside Linux (first run compiles pdfcraft; expect several minutes). For `cargo test --release`, use native Windows/macOS dev or [.github/workflows/ci.yml](../.github/workflows/ci.yml) on Ubuntu.

### Dev Container (Cursor / VS Code)

Requires **Docker Desktop** and the **Dev Containers** extension. Open the **pypdfcraft** folder, then **Reopen in Container** (or Command Palette → `Dev Containers: Reopen in Container`).

Configuration lives in [.devcontainer/devcontainer.json](../.devcontainer/devcontainer.json). It uses the `devcontainer` service in [docker/compose.yml](../docker/compose.yml), which mounts:

| Container path | Host (default) |
|----------------|----------------|
| `/work/pypdfcraft` | this repo |
| `/work/pdfcraft` | sibling `pdfcraft` checkout (`target/` uses a container volume, not your host `target/`) |

`postCreateCommand` runs `maturin develop --release` on first create. pdfcraft sources are at `/work/pdfcraft` if you need to open them in a second editor window.

If the sibling checkout is not at `D:\code\pdfcraft`, set `PDFCRAFT_ROOT` in your environment **before** reopening in container (same as compose overrides above).

### What Docker does *not* replace

- **Windows-native** `maturin develop` is still the best day-to-day loop on Windows when Rust + MSVC are installed.
- Docker builds **Linux** wheels; use CI or a Windows job for `.pyd` on Windows.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `cargo` / `rustc` not found | Restart terminal after rustup; check `%USERPROFILE%\.cargo\bin` on `PATH` |
| `link.exe` not found | Install VS Build Tools C++ workload |
| `failed to read ...\pdfcraft\...` | Fix sibling layout or junction; paths are relative to `pypdfcraft` |
| `rust-version` error | `rustup update stable` |
| Docker “pipe … dockerDesktopLinuxEngine” | Start **Docker Desktop** |
| Very long first build | Normal; pdfcraft is large. Use `sccache` locally if desired |

## CI reference

GitHub Actions workflows [.github/workflows/ci.yml](../.github/workflows/ci.yml) and [wheels.yml](../.github/workflows/wheels.yml) checkout `storytold/pdfcraft` next to `pypdfcraft` under `$GITHUB_WORKSPACE`, same layout as above.
