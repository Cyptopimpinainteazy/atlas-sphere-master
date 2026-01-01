# Atlas Sphere WASM Build Patches

This directory contains local patches for dependencies that prevent WASM compilation in Atlas Sphere. These patches provide minimal, targeted fixes to enable forkless runtime upgrades.

## Patches Overview

### errno (v0.3.9)
**Location**: `patches/errno/`
**Purpose**: Provides WASM-compatible errno implementation
**Issue**: errno v0.3.x lacks wasm32-unknown-unknown support
**Solution**: Conditional compilation providing safe stub implementations (errno=0, no-op functions) for WASM targets while preserving real libc bindings for native targets

**Files**:
- `Cargo.toml`: Conditional libc dependency
- `src/lib.rs`: WASM-safe errno stubs with conditional compilation

**Maintenance**: Update version in Cargo.toml if errno is upgraded in workspace. Test WASM builds after updates.

### rustix (v0.38.21)
**Location**: `patches/rustix/`
**Purpose**: Provides WASM-compatible rustix implementation
**Issue**: rustix depends on incompatible errno for WASM targets
**Solution**: Uses patched errno dependency with conditional re-exports and WASM guards around syscall-dependent code

**Files**:
- `Cargo.toml`: Patched errno dependency
- `src/lib.rs`: Conditional rustix re-exports

**Maintenance**: Update version in Cargo.toml if rustix is upgraded in workspace. Ensure compatibility with errno patch version.

## Usage

These patches are automatically applied via the workspace `Cargo.toml` `[patch.crates-io]` section:

```toml
[patch.crates-io]
errno = { path = "patches/errno" }
rustix = { path = "patches/rustix" }
```

## Validation

After applying patches, validate with:

```bash
# Clean build
cargo clean

# Full workspace build (includes WASM)
cargo build --release

# Direct WASM target build
cargo build --release -p atlas-sphere-runtime --target wasm32-unknown-unknown

# Runtime upgrade test
cargo test -p pallet-atlas-kernel runtime_upgrade
```

## Troubleshooting

- **Build fails**: Check that patch versions match workspace dependency versions
- **Runtime upgrades fail**: Verify WASM binary is embedded (check for `WASM_BINARY` constants in runtime)
- **Test failures**: Ensure patches don't break native functionality

## Future Maintenance

- Monitor upstream errno/rustix releases for WASM support
- Remove patches when dependencies gain native WASM support
- Update patch versions when workspace dependencies are upgraded
- Test thoroughly after any patch or dependency changes

## Related Files

- `runtime/src/lib.rs`: WASM binary inclusion (no longer conditional)
- `runtime/build.rs`: Unconditional substrate-wasm-builder execution
- `Cargo.toml`: Workspace patch configuration
- `runtime/Cargo.toml`: Removed skip-wasm-build feature
- `node/Cargo.toml`: Removed skip-wasm-build dependency