# Build pypdfcraft wheels on Linux (manylinux-compatible when using maturin-action flags).
# Build context: parent directory containing pdfcraft/ and pypdfcraft/ (see compose.yml).
# Base image is manylinux-based (no apt); Python and Rust are preinstalled.

# pdfcraft's rten 0.26 needs rustc 1.94+. Pin digest after `docker pull ghcr.io/pyo3/maturin:latest`.
FROM ghcr.io/pyo3/maturin:latest@sha256:1c897d72a79c083cdea8829662f7b28e7657cf5b0d7f44fb4231e55998cf52ed

WORKDIR /work/pypdfcraft

COPY pypdfcraft/docker/entrypoint.sh /entrypoint.sh
RUN sed -i 's/\r$//' /entrypoint.sh && chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD ["build"]
