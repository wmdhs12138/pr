## ADDED Requirements

### Requirement: Android catalog is plugin-independent

The Android app SHALL no longer require scanning `usr/etc/proot-distro/*.sh` as the source of truth for what installs can be created.

#### Scenario: Rendering the install catalog without plugins

- **WHEN** the Android app renders the install catalog for new installs
- **THEN** it does not depend on scanning bundled plugin files to decide which install options to show

### Requirement: Android app provides curated OCI presets

The Android app SHALL provide a curated preset list that maps familiar distro labels to OCI image references for common installs.

#### Scenario: Offering common distro presets

- **WHEN** the user opens the Android install screen
- **THEN** the app offers curated presets such as Alpine, Debian, and Ubuntu
- **AND** each preset maps to an OCI image reference used by the installer

### Requirement: Android app accepts custom image references

The Android app SHALL allow the user to enter a custom image reference or equivalent install source for OCI-backed installs.

#### Scenario: Installing from a custom OCI image reference

- **WHEN** the user enters a custom image reference in the Android UI
- **THEN** the app passes that reference to the OCI-backed installer flow

### Requirement: Android app shows runtime-backed install state

The Android app SHALL determine installed status and management actions from runtime-managed install state rather than from bundled asset plugin presence.

#### Scenario: Showing installed state for OCI-backed installs

- **WHEN** an OCI-backed install exists on disk
- **THEN** the Android UI shows it as installed even if no matching plugin asset exists

#### Scenario: Showing installed state for legacy installs during migration

- **WHEN** a legacy plugin-based install exists on disk
- **THEN** the Android UI still shows it as a manageable installed entry during the migration period
