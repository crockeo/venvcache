release-all: release-macos-aarch64 release-linux-aarch64 release-linux-x86-64

release-macos-aarch64:
    # Assumes that you're on macOS,
    # because I'm on macOS :)
    cargo build --release

release-linux-aarch64:
    cross build \
        --release \
        --target=aarch64-unknown-linux-musl \
        --target-dir=target-linux-aarch64

release-linux-x86-64:
    cross build \
        --release \
        --target=x86_64-unknown-linux-musl \
        --target-dir=target-linux-x86-64
