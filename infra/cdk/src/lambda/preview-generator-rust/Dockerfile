FROM rust:bullseye AS builder

# Install cross-compilation toolchain for x86_64 Linux
RUN apt-get update && apt-get install -y --no-install-recommends gcc-x86-64-linux-gnu libc6-dev-amd64-cross && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy project files
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build for x86_64-unknown-linux-gnu (AWS Lambda runtime)
RUN rustup target add x86_64-unknown-linux-gnu
ENV CFLAGS_x86_64_unknown_linux_gnu="-I/usr/include/x86_64-linux-gnu" \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
RUN CC=x86_64-linux-gnu-gcc cargo build --release --target x86_64-unknown-linux-gnu

# Verify the binary
RUN file target/x86_64-unknown-linux-gnu/release/bootstrap || echo "Binary not found, checking..." && ls target/x86_64-unknown-linux-gnu/release/ || echo "Target directory not found"
