# venvcache

Provides a safe mechanism to cache Python venvs on disk with:

- Automatic LRU cache eviction.
- Safety across multiple simultaneous invocations.

Designed to be used in CI.

## Cross-Compilation

- We use [cross](https://github.com/cross-rs/cross) to cross-compile.
  Install that per its installation instructions.

- Ensure Docker is running.

- `just release-linux-aarch64` for aarch64, `just release-linux-x86-64` for x86_64.

## License

MIT Open Source, see [LICENSE](./LICENSE).
