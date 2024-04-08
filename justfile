release-all: release-macos-aarch64 release-linux-aarch64 release-linux-x86-64

release-macos-aarch64:
    # Assumes that you're on macOS,
    # because I'm on macOS :)
    cargo build --release

release-linux-aarch64:
    docker run \
        --platform linux/arm64 \
        --rm \
        --user "$(id -u)":"$(id -g)" \
        -v "$PWD":/usr/src/myapp \
        -w /usr/src/myapp \
        rust:latest \
        cargo build \
            --release \
            --target-dir=target-linux-aarch64

release-linux-x86-64:
    docker run \
        --platform linux/amd64 \
        --rm \
        --user "$(id -u)":"$(id -g)" \
        -v "$PWD":/usr/src/myapp \
        -w /usr/src/myapp \
        rust:latest \
        cargo build \
            --release \
            --target-dir=target-linux-x86-64
