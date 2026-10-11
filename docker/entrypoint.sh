#!/bin/sh
set -euo pipefail

cd /work/pypdfcraft

if [ ! -f ../pdfcraft/Cargo.toml ]; then
  echo "error: ../pdfcraft not found. Mount pdfcraft at /work/pdfcraft (sibling of pypdfcraft)." >&2
  exit 1
fi

PYTHON="${PYTHON:-/opt/python/cp312-cp312/bin/python}"

# Compose mounts a Linux-only volume at .venv (do not rm the mount point).
if [ ! -x .venv/bin/python ]; then
  if [ -d .venv ]; then
    find .venv -mindepth 1 -delete 2>/dev/null || true
  else
    mkdir -p .venv
  fi
  "${PYTHON}" -m venv .venv
fi
export VIRTUAL_ENV=/work/pypdfcraft/.venv
export PATH="${VIRTUAL_ENV}/bin:${PATH}"

python -m pip install -q -U pip maturin pytest

cmd="${1:-build}"
shift || true

case "$cmd" in
  build)
    maturin build --release --out dist --compatibility pypi "$@"
    ls -la dist/
    ;;
  develop)
    maturin develop --release "$@"
    ;;
  test)
    maturin develop --release
    pytest "$@"
    # Plain `cargo test` needs libpython on the linker path; maturin develop already
    # compiled the extension. Run `cargo test` on the host or in GitHub Actions CI.
    ;;
  shell)
    exec sh
    ;;
  idle)
    exec sleep infinity
    ;;
  *)
    echo "usage: build | develop | test | shell | idle" >&2
    exit 2
    ;;
esac
