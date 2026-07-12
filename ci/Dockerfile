FROM rust:1.96.0-bookworm@sha256:5e2214abe154fe26e39f64488952e5c991eeed1d6d6da7cc8381ae83927f0cfc AS rust-tools

RUN rustup toolchain install 1.88.0 --profile minimal --component clippy,rustfmt \
    && rustup toolchain install 1.96.0 --profile minimal --component clippy,rustfmt \
    && rustup default 1.96.0 \
    && cargo install --locked --version 0.22.2 cargo-audit \
    && cargo install --locked --version 0.19.9 cargo-deny \
    && cargo install --locked --version 0.9.2 cargo-machete \
    && cargo install --locked --version 0.10.0 taplo-cli

RUN rm -rf /usr/local/cargo/registry /usr/local/cargo/git \
    && mkdir -p /opt/compiler/bin /opt/compiler/lib /opt/sysroot \
    && for tool in cc gcc as ld ar ranlib strip; do cp -L "$(command -v "$tool")" "/opt/compiler/bin/$tool"; done \
    && for object in Scrt1.o crti.o crtn.o; do cp -L "$(gcc -print-file-name="$object")" "/opt/compiler/lib/$object"; done \
    && for library in libc.so libm.so libutil.a librt.a libpthread.a libdl.a; do cp "$(gcc -print-file-name="$library")" "/opt/compiler/lib/$library"; done \
    && libc_dir="$(dirname "$(gcc -print-file-name=libc.so)")" \
    && mkdir -p "/opt/sysroot$libc_dir" \
    && cp -L "$(gcc -print-file-name=libc_nonshared.a)" "/opt/sysroot$libc_dir/libc_nonshared.a" \
    && for binary in "$(command -v ld)" "$(command -v as)" "$(gcc -print-prog-name=cc1)"; do ldd "$binary"; done \
      | awk '/=> \// && $1 !~ /^lib(c|m|z|zstd)\./ {print $3}' | sort -u | xargs cp -L -t /opt/compiler/lib

FROM mcr.microsoft.com/playwright@sha256:57b65fdc9ceabe0ef613124c7bbe2babcf9362c4d85e382fe3b03604e84b428a

COPY --from=rust-tools /usr/local/cargo /usr/local/cargo
COPY --from=rust-tools /usr/local/rustup /usr/local/rustup
COPY --from=rust-tools /usr/bin/make /usr/bin/make
COPY --from=rust-tools /opt/compiler/bin /usr/bin
COPY --from=rust-tools /opt/compiler/lib /usr/local/compiler/lib
COPY --from=rust-tools /usr/lib/gcc /usr/lib/gcc
COPY --from=rust-tools /usr/include /usr/include
COPY --from=rust-tools /opt/sysroot/ /

ENV PATH=/usr/local/cargo/bin:${PATH} \
    LD_LIBRARY_PATH=/usr/local/compiler/lib \
    LIBRARY_PATH=/usr/local/compiler/lib \
    RUSTUP_HOME=/usr/local/rustup \
    RUST_BACKTRACE=1 \
    PLAYWRIGHT_BROWSERS_PATH=/ms-playwright

RUN node --version | grep -Eq '^v24\.' \
    && command -v git make python3 bash cc \
    && rustc +1.96.0 --version | grep -q '1.96.0' \
    && rustc +1.88.0 --version | grep -q '1.88.0'

WORKDIR /work
