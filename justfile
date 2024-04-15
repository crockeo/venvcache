release-all: release-macos-aarch64 release-linux-aarch64 release-linux-x86-64

release-macos-aarch64:
    # Assumes that you're on macOS,
    # because I'm on macOS :)
    cargo build --release
    mkdir -p releases/
    mv target/release/venvcache releases/venvcache-macos-aarch64

release-linux-aarch64:
    cross build \
        --release \
        --target=aarch64-unknown-linux-musl
    mkdir -p releases/
    mv target/aarch64-unknown-linux-musl/release/venvcache releases/venvcache-linux-aarch64

release-linux-x86-64:
    cross build \
        --release \
        --target=x86_64-unknown-linux-musl
    mkdir -p releases/
    mv target/x86_64-unknown-linux-musl/release/venvcache releases/venvcache-linux-x86-64
