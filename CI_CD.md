# CI/CD Pipeline Documentation

This document describes the continuous integration and continuous deployment (CI/CD) pipelines for the haproxy-otel project.

## Overview

The project uses GitHub Actions for automated testing, building, and releasing across multiple platforms.

## Workflows

### Main CI Workflow (`.github/workflows/main.yaml`)

Triggered on every push and pull request.

#### Jobs

1. **build** - Matrix build job that tests and builds for all platforms
   - **Test on Linux x86_64**: 
     - Sets up Rust toolchain
     - Installs HAProxy 3.2 from PPA
     - Runs test suite: `cargo test -p haproxy-otel-tests`
     - Builds the module
   - **Cross-platform builds**:
     - Linux x86_64 (native build with tests)
     - Linux ARM64 (cross-compilation using `cross`)
     - macOS x86_64 (native build)
     - macOS ARM64 (native build)
   - Uploads build artifacts for each platform

2. **rustfmt** - Checks code formatting
   - Uses nightly Rust toolchain
   - Validates with `cargo fmt -- --check`

3. **clippy** - Runs Rust linter
   - Uses nightly Rust toolchain
   - Reports issues through GitHub PR reviews

### Release Workflow (`.github/workflows/release.yaml`)

Triggered on tag creation (tags matching `v*` pattern, e.g., `v0.2.0`).

#### Jobs

1. **build-release** - Matrix build for all platforms
   - Builds optimized release binaries for all platforms:
     - `libhaproxy_otel_module-linux-x86_64.so`
     - `libhaproxy_otel_module-linux-aarch64.so`
     - `libhaproxy_otel_module-macos-x86_64.dylib`
     - `libhaproxy_otel_module-macos-aarch64.dylib`
   - Strips debug symbols from binaries
   - Uploads artifacts for the release

2. **create-release** - Creates a GitHub Release
   - Downloads all build artifacts
   - Creates release with auto-generated notes
   - Attaches all platform binaries

3. **publish-crate** (Optional) - Publishes to crates.io
   - Only runs for the `caseware` organization
   - Requires `CARGO_TOKEN` secret
   - Continues on error if already published or token not set

## Supported Platforms

| Platform | Architecture | File Extension | Runner |
|----------|--------------|----------------|--------|
| Linux | x86_64 | `.so` | ubuntu-latest |
| Linux | ARM64 | `.so` | ubuntu-latest (cross) |
| macOS | x86_64 | `.dylib` | macos-latest |
| macOS | ARM64 | `.dylib` | macos-latest |

## Creating a Release

To create a new release:

1. Update the version in `Cargo.toml`
2. Commit your changes
3. Create and push a tag:
   ```bash
   git tag v0.3.0
   git push origin v0.3.0
   ```
4. The release workflow will automatically:
   - Build binaries for all platforms
   - Create a GitHub Release
   - Upload all artifacts
   - (Optionally) Publish to crates.io

## Build Artifacts

All builds produce artifacts that are uploaded and available for download:

- **CI Builds**: Available for 90 days in the Actions tab
- **Release Builds**: Permanently attached to GitHub Releases

## Local Development

To build locally for your platform:

```bash
# Build the module in debug mode
cargo build -p haproxy-otel-module

# Build the module in release mode
cargo build --release -p haproxy-otel-module

# Run tests (requires HAProxy 3.2)
cargo test -p haproxy-otel-tests

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Cross-Compilation

For cross-compilation (e.g., Linux ARM64 on x86_64):

```bash
# Install cross
cargo install cross --git https://github.com/cross-rs/cross

# Build for ARM64
cross build --release --target aarch64-unknown-linux-gnu -p haproxy-otel-module
```

## Secrets Required

For the release workflow to work completely, the following secrets need to be configured in the repository settings:

- `CARGO_TOKEN` (Optional): Token for publishing to crates.io
  - Get this from https://crates.io/me
  - Only needed if you want automatic crates.io publishing

## Troubleshooting

### Build Failures

- Check that all Rust dependencies are compatible with the target platform
- Ensure cross-compilation tools are properly installed
- Review the build logs in the Actions tab

### Test Failures

- Tests require HAProxy 3.2 to be installed
- Ensure the mock server ports (4317, 8080) are available
- Check that the HAProxy configuration files in `tests/` are valid

### Release Issues

- Ensure the tag follows the `v*` pattern (e.g., `v1.0.0`)
- Verify that the version in `Cargo.toml` matches the tag
- Check that GitHub Actions has permission to create releases
