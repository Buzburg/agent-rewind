# Release verification — agent-rewind v0.1.1

Verified locally on October 27, 2026, Linux x86-64. Toolchains: Python 3.14.8, Rust 1.98.1, Node.js 26.8.1, Go 1.27.1 and Mojo 1.1.0, as applicable. CI covers additional versions/platforms where configured; consult the workflow run for the release commit rather than an older green run.

## Checks

```sh
cargo test --locked
cargo build --release --locked
```

**Local result: PASS.** 12 recovery/history tests; release-binary save → modify/delete/add → diff → restore acceptance; wrapped child exit status preserved.

The Python build command requires the `build` package; Unity checks also require `mypy` and `ruff`. Native Mojo experiments are optional and require Mojo 1.1.0. CLI acceptance used disposable fixtures and no real customer data.

## Release changes

Preserve recovered executable scripts and unrelated empty directories; propagate traversal errors; replace manifests through a synced temporary file.

The README and repository summary now explain the supported behavior, installation, and limits without unverified speed or security promises.

## Limits

Checksums, path binding, symlink/hard-link boundaries, corrupt/missing blobs and executable-script recovery are covered. Whole-workspace atomicity, concurrent writers and full metadata restoration are not provided.

Git history was checked for common credential/key patterns. Matches in Envghost were synthetic scanner fixtures; no live credentials were identified by that scan. This is a limited pattern check, not a comprehensive security certification.
