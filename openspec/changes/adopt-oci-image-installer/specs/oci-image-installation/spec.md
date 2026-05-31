## ADDED Requirements

### Requirement: OCI-backed install sources

`pr-cli install` SHALL accept OCI/Docker image references, explicit registry-qualified image references, local OCI or rootfs archives, and direct archive URLs as install sources for new guest environments.

#### Scenario: Installing from a short image reference

- **WHEN** the user runs `pr-cli install alpine:latest`
- **THEN** the installer treats `alpine:latest` as an OCI image reference
- **AND** records the normalized source reference in install metadata

#### Scenario: Installing from an explicit registry-qualified image reference

- **WHEN** the user runs `pr-cli install docker.io/library/debian:stable`
- **THEN** the installer resolves that fully qualified OCI image reference directly

#### Scenario: Installing from an archive source

- **WHEN** the user provides a supported local archive path or direct archive URL
- **THEN** the installer uses that archive as the install source without requiring a bundled plugin file

### Requirement: OCI architecture resolution

For OCI image references, the installer SHALL resolve image manifests and select the best matching artifact for the current device architecture, failing with a user-visible error when no supported image is available.

#### Scenario: Selecting a matching arm64 image

- **WHEN** the current device architecture is arm64 and the source image publishes a matching manifest
- **THEN** the installer selects the arm64-compatible image automatically

#### Scenario: Rejecting an unsupported architecture

- **WHEN** the source image does not publish a supported image for the current device architecture
- **THEN** the install command fails with an explicit unsupported-architecture error

### Requirement: OCI layer extraction

For OCI image installs, the installer SHALL download all required layers, apply them in order into the target rootfs, and honor OCI whiteout semantics while preserving filesystem attributes required for a working guest rootfs.

#### Scenario: Extracting a layered rootfs

- **WHEN** the selected OCI image contains multiple filesystem layers
- **THEN** the installer downloads and applies those layers in manifest order into the target rootfs

#### Scenario: Applying OCI whiteouts

- **WHEN** a later OCI layer contains whiteout entries
- **THEN** the installer removes or masks files from lower layers according to OCI whiteout rules

### Requirement: OCI install metadata layout

OCI-backed installs SHALL be stored under `${APP_PREFIX}/var/lib/proot-distro/containers/<name>/` with a `rootfs/` directory and a `manifest.json` file containing at least the install name, source kind, original source reference, normalized source reference, selected architecture, and creation timestamp.

#### Scenario: Writing manifest metadata

- **WHEN** an OCI-backed install completes successfully
- **THEN** the installer writes `manifest.json` beside the new `rootfs/`
- **AND** the manifest records the normalized install source and selected architecture

### Requirement: Android runtime setup after OCI install

After OCI extraction completes, `pr` SHALL apply the same Android-specific guest runtime setup currently required for login and command execution.

#### Scenario: Reusing Android-specific guest setup

- **WHEN** a rootfs is installed from an OCI image
- **THEN** the post-install flow still applies Android-specific runtime setup needed by `pr`
- **AND** the resulting guest can be used with the existing login mechanism

### Requirement: Host-side install management is Rust-owned

The OCI-backed install and management flow SHALL be implemented in `pr-cli` without requiring a bundled host Bash binary such as `libbash.so` for install, list, login, remove, reset, backup, restore, or test operations.

#### Scenario: Managing OCI installs without bundled Bash

- **WHEN** the app manages an OCI-backed install
- **THEN** the host-side flow executes through Rust-owned logic and recorded metadata
- **AND** it does not require `libbash.so` to be packaged in the app

### Requirement: OCI-backed install enumeration and reset

`pr-cli list` and reset or reinstall flows SHALL enumerate and manage OCI-backed installs from runtime metadata without requiring a matching plugin file.

#### Scenario: Listing OCI-backed installs

- **WHEN** a container exists under the OCI-backed install layout
- **THEN** `pr-cli list` includes that install even if no distro plugin file exists

#### Scenario: Resetting an OCI-backed install

- **WHEN** the user resets an OCI-backed install
- **THEN** the command recreates it from recorded install metadata rather than from a plugin file
