## Context

`pr` currently implements a forked `proot-distro` v4-style workflow: Android bundles plugin shell scripts, `pr-cli` parses those plugins, and installs download a single tarball per distro/architecture. Upstream `proot-distro` v5 moved away from bundled shell plugins to a generic OCI/container-image installer written in Python. This project cannot adopt upstream v5 verbatim because `pr` is Rust-based, Android-specific, and already has custom runtime behaviors around fake root, fake kernel release, and guest environment setup.

The desired change is architectural: keep `pr`'s Android runtime behavior, but replace the plugin/tarball source model with OCI-aware installation and metadata management.

## Goals / Non-Goals

**Goals:**
- Add OCI-backed install support to `pr-cli` in Rust
- Decouple install and list flows from bundled plugin files
- Preserve Android-specific guest setup after extraction
- Keep existing plugin-based installs usable during migration
- Replace the Android distro catalog with OCI-aware presets plus custom references
- Remove the bundled `libbash.so` dependency by porting remaining host-side distro-management shell logic into Rust

**Non-Goals:**
- Port the upstream Python implementation directly into the app
- Remove legacy plugin support in the first OCI release
- Implement every upstream v5 feature immediately, such as full registry auth UX or every possible archive format
- Migrate every existing install automatically in the first step
- Eliminate every shell script in the repository, including bootstrap and developer tooling, in the first OCI milestone

## Decisions

### 1. Port the mechanism, not the implementation

`pr` will keep `pr-cli` as the installer/control plane and reimplement the OCI flow in Rust. This avoids introducing a Python runtime into the Android app and preserves the project's existing control over Android-specific behavior.

**Alternatives considered:**
- Embed upstream Python v5 directly: rejected because it mismatches the current app/runtime architecture
- Shell out to an external OCI tool: rejected because Android packaging, SELinux, and offline behavior become harder to control

### 2. Use a dual-layout transition

OCI-backed installs will live in a new container-oriented layout under `var/lib/proot-distro/containers/<name>/`, while legacy plugin installs continue to use the existing layout during migration.

**Alternatives considered:**
- In-place conversion of the legacy layout: rejected for the first phase because it increases migration risk
- Hard cut-over with no legacy support: rejected because it would break existing user installs

### 3. Replace plugin files with explicit metadata

Each OCI-backed install will write a `manifest.json` that records source kind, normalized image reference, selected architecture, and install identity. Management commands will use this metadata instead of discovering state from plugin files.

**Alternatives considered:**
- Continue generating synthetic plugin files for OCI installs: rejected because it keeps the old abstraction alive and complicates management semantics

### 4. Move Android UI to preset-plus-custom input

The Android app will no longer derive the install catalog from bundled plugin assets. Instead it will ship a small preset list for common distributions and allow custom image references for advanced installs.

**Alternatives considered:**
- Freeform-only UI: rejected because it weakens discoverability
- Presets only: rejected because it does not match the flexibility of the OCI model

### 5. Keep source-aware command behavior

Commands such as `list`, `remove`, `reset`, `backup`, and `restore` will branch on install source type. Legacy installs continue to use plugin-derived metadata; OCI installs use recorded manifest metadata.

**Alternatives considered:**
- Force all commands through a migration prerequisite: rejected because it makes adoption more disruptive

### 6. Remove bundled host Bash as part of the migration end state

The OCI migration will treat `libbash.so` removal as an explicit design objective, not as optional cleanup. Host-side install and management behavior will live in Rust (`pr-cli`) with structured metadata instead of Bash-driven plugin execution. Any remaining guest setup hooks that must survive the transition should be represented as Rust-managed operations or as POSIX-`sh` guest commands launched intentionally inside the installed rootfs, not by depending on a bundled host Bash binary.

**Alternatives considered:**
- Keep shipping `libbash.so` indefinitely as a dormant compatibility binary: rejected because it preserves packaging and maintenance cost without being part of the target architecture
- Replace Bash with another host shell while keeping shell-driven install logic: rejected because the goal is to converge on Rust-managed install logic rather than swap one host shell dependency for another

### 7. Use task-scope coverage gates during migration

Coverage enforcement for this change will be scoped to the modules touched by each OCI task slice, not to global repository coverage in early phases. Each implemented task must keep a minimum of 80% line coverage across its directly affected OCI-focused Rust modules.

**Alternatives considered:**
- Require global `pr-cli` coverage >= 80% immediately: rejected because migration starts with foundational slices while many unrelated modules are not in scope yet
- Skip coverage thresholds and rely only on scenario tests: rejected because parser/resolver/extractor regressions are easier to miss without a numeric gate

## Risks / Trade-offs

- **[OCI extraction complexity]** → Whiteouts, layer ordering, and tar edge cases can produce subtly broken rootfs trees. Mitigation: build the layer application code behind focused tests and start with a narrow set of supported images.
- **[Registry protocol scope]** → Full registry auth and manifest handling can grow large. Mitigation: begin with anonymous pulls for common public registries and a limited source matrix.
- **[Dual-layout maintenance cost]** → Supporting both legacy and OCI installs increases branching in command logic. Mitigation: introduce a shared install descriptor model early so downstream commands do not care about storage details more than necessary.
- **[Shell-porting scope]** → Replacing residual shell-based install logic may uncover distro-specific assumptions currently hidden in plugin hooks. Mitigation: identify the remaining hook surface early, move generic behavior into Rust first, and keep compatibility behavior explicit and testable.
- **[UI migration confusion]** → Users may not understand why presets differ from old distro names. Mitigation: present presets with familiar distro labels and keep existing installed distros visible.
- **[Performance/storage regression]** → OCI installs involve multiple blobs and metadata files instead of one tarball. Mitigation: use download caching and reuse existing cache lifecycle commands where possible.
- **[False confidence from broad global coverage numbers]** → A global metric can hide weak coverage in the exact slice being changed. Mitigation: enforce >= 80% line coverage on task-scoped OCI modules and keep scenario tests for integration commands.

## Migration Plan

1. Introduce internal install descriptors that can represent both legacy and OCI-backed installs.
2. Add OCI install source parsing, manifest resolution, and layer extraction behind a new install path.
3. Add OCI container metadata and new storage layout while preserving the existing layout.
4. Teach list/login/remove/reset/backup/restore/test commands to work with both source types.
5. Port remaining host-side shell-driven distro-management behavior into Rust and remove the need to ship `libbash.so`.
6. Replace Android plugin-scanned catalog UI with OCI presets and custom image entry.
7. Deprecate bundled plugin assets after OCI install flows and compatibility paths are stable.

Rollback for early releases is straightforward: disable the OCI install entry points while leaving legacy plugin installs untouched.

## Open Questions

- Which registries and archive formats are required in the first milestone?
- Should preset names preserve release labels such as `Debian (stable)` or stay generic and let the image ref carry the exact version?
- Should backup/restore for OCI installs preserve raw layer metadata, or only rootfs plus normalized manifest information?
- Do we want a dedicated user-visible migration command once OCI installs are proven stable?
- Do any distro-specific setup steps need a declarative hook model after the Rust migration, or can they all be absorbed into shared Rust install logic?
