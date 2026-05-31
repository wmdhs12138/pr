## 1. OCI install foundations

- [x] 1.1 Define a shared install descriptor model that can represent both legacy plugin installs and OCI-backed installs
- [x] 1.2 Add storage helpers for `var/lib/proot-distro/containers/<name>/rootfs` and `manifest.json`
- [x] 1.3 Implement source parsing for OCI image references, direct URLs, and local archives
- [x] 1.4 Implement manifest resolution and architecture selection for public OCI registries
- [x] 1.5 Implement OCI layer download, extraction, and whiteout handling into a target rootfs

## 2. CLI command migration

- [x] 2.1 Refactor `install` to support OCI-backed installs while preserving Android-specific guest setup
- [x] 2.2 Refactor `list` to enumerate both legacy installs and OCI-backed containers
- [x] 2.3 Refactor `login`, `remove`, `reset`, `copy`, `backup`, `restore`, and `test` to branch on install source type
- [x] 2.4 Persist and load OCI install metadata through `manifest.json`
- [x] 2.5 Port remaining host-side distro-management shell behavior from plugin/setup flows into Rust-owned logic
- [x] 2.6 Add targeted tests for OCI source parsing, manifest selection, metadata loading, dual-layout enumeration, and Rust-owned setup behavior

## 3. Android app migration

- [x] 3.1 Remove the plugin-scanned distro catalog as the Android UI source of truth
- [x] 3.2 Add curated OCI presets for Alpine, Debian, Ubuntu, and other supported defaults
- [x] 3.3 Add custom image reference input in the Android UI
- [x] 3.4 Drive installed status and management actions from runtime install state instead of bundled assets
- [x] 3.5 Stop packaging `libbash.so` once app/runtime code no longer requires a bundled host Bash binary

## 4. Compatibility and rollout

- [ ] 4.1 Keep existing plugin-based installs working without mandatory migration
- [ ] 4.2 Define source-aware backup, restore, and reset behavior for OCI-backed installs
- [ ] 4.3 Update user-facing docs and internal OpenSpec specs once the OCI path is implemented
- [ ] 4.4 Remove or deprecate bundled plugin assets after OCI installs are stable
- [ ] 4.5 Remove the remaining legacy `proot-distro.sh`/host-shell assumptions after the Rust migration is complete

## 5. Coverage gates

- [ ] 5.1 Enforce task-scope Rust line coverage >= 80% for each OCI migration slice before marking the task complete
- [ ] 5.2 Document and run per-slice coverage commands alongside targeted tests for each completed task group
