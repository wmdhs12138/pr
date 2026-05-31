## ADDED Requirements

### Requirement: Existing plugin-based installs remain usable

Existing legacy installs created from plugin-defined tarballs SHALL remain usable during the OCI transition for login, remove, copy, backup, restore, and test flows.

#### Scenario: Logging into a legacy install after OCI support lands

- **WHEN** the user already has a legacy plugin-based Debian install on disk
- **THEN** the user can still log in to that install after OCI-backed install support is introduced

### Requirement: Management commands support both install types

Management commands SHALL enumerate and manage both legacy plugin-based installs and OCI-backed installs during the transition period.

#### Scenario: Listing legacy and OCI installs together

- **WHEN** the device contains both a legacy plugin-based install and an OCI-backed install
- **THEN** `pr-cli list` includes both installs

#### Scenario: Removing an OCI install without a plugin file

- **WHEN** the user removes an OCI-backed install
- **THEN** the command uses recorded OCI metadata
- **AND** does not require a matching plugin file

#### Scenario: Removing a legacy install without OCI metadata

- **WHEN** the user removes a legacy plugin-based install
- **THEN** the command continues to work without requiring OCI metadata files

### Requirement: Migration remains optional in the first OCI release

The initial OCI-backed release SHALL not require automatic or mandatory migration of existing legacy installs before they can be used.

#### Scenario: Parallel operation during migration

- **WHEN** the project first ships OCI-backed install support
- **THEN** existing legacy installs remain usable as-is
- **AND** users may create OCI-backed installs alongside them

### Requirement: Legacy compatibility does not preserve host Bash as a permanent dependency

Legacy compatibility during the OCI transition SHALL not require the project to keep bundled host Bash as part of the intended end-state architecture.

#### Scenario: Keeping legacy installs usable while removing bundled Bash

- **WHEN** the project finishes migrating host-side distro-management logic into Rust
- **THEN** existing legacy installs can still be discovered and managed through compatibility logic
- **AND** that compatibility does not rely on shipping `libbash.so`
