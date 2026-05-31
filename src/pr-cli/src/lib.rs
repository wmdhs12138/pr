pub mod cmd_test;
pub mod color;
pub mod commands_extra;
pub mod install;
pub mod install_model;
pub mod login;
pub mod oci;
pub mod plugin;
pub mod source_parse;
pub mod shared;

pub use install_model::{InstallDescriptor, InstallSource, InstallSourceKind};
pub use oci::{
    normalize_architecture, oci_architecture_name, project_architecture_name,
    select_manifest_descriptor, NormalizedArchitecture, OciDescriptor, OciImageIndex,
    OciManifest, OciPlatform,
};
pub use source_parse::{InstallSourceInput, InstallSourceInputKind};
