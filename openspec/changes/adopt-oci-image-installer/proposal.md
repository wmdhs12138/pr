## Why

`pr` currently depends on bundled shell plugin files and prebuilt rootfs tarballs to define and install distributions. It also still carries legacy shell-era runtime pieces such as bundled `libbash.so` even though current Android runtime behavior is increasingly implemented in Rust. Upstream `proot-distro` has moved to an OCI image model, and following that direction would let `pr` install from standard container registries and archives instead of maintaining a growing plugin catalog.

## What Changes

- Replace the plugin-driven install source model in `pr-cli` with an OCI/container image install path.
- Add image-reference based installs for common registries plus local archive and direct URL sources.
- Introduce container metadata and storage layout that does not require a matching plugin file to manage an installation.
- Move remaining host-side distro management logic from shell assets into Rust so install and management flows no longer depend on bundled Bash.
- Update Android UI flows so available installs come from curated image presets and/or explicit image references rather than scanning `assets/plugins/*.sh`.
- Preserve Android-specific runtime behavior and a compatibility path for existing plugin-based installations during migration.
- **BREAKING**: new installs will no longer require or depend on bundled distro plugin files as the source of truth.
- **BREAKING**: the Android app will stop bundling `libbash.so` once remaining distro-management shell paths are replaced by Rust implementations.

## Capabilities

### New Capabilities
- `oci-image-installation`: Install and manage guest root filesystems from OCI image references, OCI archives, or direct archive URLs.
- `legacy-plugin-compatibility`: Keep existing plugin-based installations usable while the project transitions to OCI-backed installs.
- `android-image-catalog`: Replace Android's plugin-scanned distro list with OCI-aware presets and custom image entry.

### Modified Capabilities

None. There are currently no active capability specs in `openspec/specs/` to modify.

## Impact

- `src/pr-cli/src/main.rs`
- `src/pr-cli/src/install.rs`
- `src/pr-cli/src/plugin.rs`
- `src/pr-cli/src/commands_extra.rs`
- `src/pr-cli/src/shared.rs`
- `android/app/src/main/java/id/or/oo/pr/App.kt`
- `android/app/src/main/java/id/or/oo/pr/MainActivity.kt`
- `android/app/src/main/jniLibs/arm64-v8a/libbash.so`
- `src/scripts/proot-distro.sh`
- install storage under `usr/var/lib/proot-distro/`
- distro asset packaging under `android/app/src/main/assets/plugins/`
