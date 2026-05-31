use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::plugin::DistroPlugin;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallSourceKind {
    LegacyPlugin,
    OciImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallSource {
    LegacyPlugin {
        plugin_alias: String,
        plugin_path: PathBuf,
        architecture: String,
    },
    OciImage {
        image_reference: String,
        architecture: Option<String>,
    },
}

impl InstallSource {
    pub fn kind(&self) -> InstallSourceKind {
        match self {
            InstallSource::LegacyPlugin { .. } => InstallSourceKind::LegacyPlugin,
            InstallSource::OciImage { .. } => InstallSourceKind::OciImage,
        }
    }

    pub fn architecture(&self) -> Option<&str> {
        match self {
            InstallSource::LegacyPlugin { architecture, .. } => Some(architecture.as_str()),
            InstallSource::OciImage { architecture, .. } => architecture.as_deref(),
        }
    }

    pub fn source_ref(&self) -> &str {
        match self {
            InstallSource::LegacyPlugin { plugin_alias, .. } => plugin_alias.as_str(),
            InstallSource::OciImage { image_reference, .. } => image_reference.as_str(),
        }
    }

    pub fn source_path(&self) -> Option<&Path> {
        match self {
            InstallSource::LegacyPlugin { plugin_path, .. } => Some(plugin_path.as_path()),
            InstallSource::OciImage { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallDescriptor {
    pub name: String,
    pub rootfs_path: PathBuf,
    pub metadata_path: Option<PathBuf>,
    pub source: InstallSource,
}

impl InstallDescriptor {
    pub fn legacy_plugin(
        name: impl Into<String>,
        rootfs_path: impl Into<PathBuf>,
        plugin_alias: impl Into<String>,
        plugin_path: impl Into<PathBuf>,
        architecture: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            rootfs_path: rootfs_path.into(),
            metadata_path: None,
            source: InstallSource::LegacyPlugin {
                plugin_alias: plugin_alias.into(),
                plugin_path: plugin_path.into(),
                architecture: architecture.into(),
            },
        }
    }

    pub fn oci_image(
        name: impl Into<String>,
        rootfs_path: impl Into<PathBuf>,
        metadata_path: impl Into<PathBuf>,
        image_reference: impl Into<String>,
        architecture: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            rootfs_path: rootfs_path.into(),
            metadata_path: Some(metadata_path.into()),
            source: InstallSource::OciImage {
                image_reference: image_reference.into(),
                architecture,
            },
        }
    }

    pub fn kind(&self) -> InstallSourceKind {
        self.source.kind()
    }

    pub fn architecture(&self) -> Option<&str> {
        self.source.architecture()
    }

    pub fn source_ref(&self) -> &str {
        self.source.source_ref()
    }

    pub fn metadata_path(&self) -> Option<&Path> {
        self.metadata_path.as_deref()
    }

    pub fn from_legacy_plugin(
        plugin: &DistroPlugin,
        rootfs_path: impl Into<PathBuf>,
        plugin_path: impl Into<PathBuf>,
        architecture: impl Into<String>,
    ) -> Self {
        Self::legacy_plugin(
            plugin.name.clone(),
            rootfs_path,
            plugin.alias.clone(),
            plugin_path,
            architecture,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OciInstallMetadata {
    pub install_name: String,
    pub source_kind: String,
    pub original_source_reference: String,
    pub normalized_source_reference: String,
    pub selected_architecture: String,
    pub created_at: u64,
}

impl OciInstallMetadata {
    pub fn new(
        install_name: impl Into<String>,
        original_source_reference: impl Into<String>,
        normalized_source_reference: impl Into<String>,
        selected_architecture: impl Into<String>,
        created_at: u64,
    ) -> Self {
        Self {
            install_name: install_name.into(),
            source_kind: "oci-image".to_string(),
            original_source_reference: original_source_reference.into(),
            normalized_source_reference: normalized_source_reference.into(),
            selected_architecture: selected_architecture.into(),
            created_at,
        }
    }
}

pub fn write_oci_install_metadata(
    metadata_path: &Path,
    metadata: &OciInstallMetadata,
) -> Result<(), String> {
    let Some(parent) = metadata_path.parent() else {
        return Err(format!(
            "invalid OCI metadata path {}",
            metadata_path.display()
        ));
    };
    fs::create_dir_all(parent)
        .map_err(|e| format!("create OCI metadata dir {}: {}", parent.display(), e))?;
    let bytes = serde_json::to_vec_pretty(metadata)
        .map_err(|e| format!("serialize OCI metadata {}: {}", metadata_path.display(), e))?;
    fs::write(metadata_path, bytes)
        .map_err(|e| format!("write OCI metadata {}: {}", metadata_path.display(), e))
}

pub fn load_oci_install_metadata(metadata_path: &Path) -> Result<OciInstallMetadata, String> {
    let content = fs::read_to_string(metadata_path)
        .map_err(|e| format!("read OCI metadata {}: {}", metadata_path.display(), e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("parse OCI metadata {}: {}", metadata_path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn legacy_plugin_descriptor_exposes_common_fields() {
        let plugin = DistroPlugin {
            alias: "alpine".to_string(),
            name: "Alpine Linux".to_string(),
            comment: None,
            tarballs: Default::default(),
            has_setup: false,
        };
        let descriptor = InstallDescriptor::legacy_plugin(
            "Alpine Linux",
            "/data/data/id.or.oo.pr/files/usr/var/lib/proot-distro/installed-rootfs/alpine",
            "alpine",
            "/data/data/id.or.oo.pr/files/usr/share/proot-distro/plugins/alpine.sh",
            "aarch64",
        );

        assert_eq!(descriptor.name, "Alpine Linux");
        assert_eq!(descriptor.kind(), InstallSourceKind::LegacyPlugin);
        assert_eq!(descriptor.source_ref(), "alpine");
        assert_eq!(descriptor.architecture(), Some("aarch64"));
        assert_eq!(descriptor.metadata_path(), None);
        assert_eq!(
            descriptor.source.source_path(),
            Some(Path::new("/data/data/id.or.oo.pr/files/usr/share/proot-distro/plugins/alpine.sh"))
        );

        let converted = InstallDescriptor::from_legacy_plugin(
            &plugin,
            "/data/data/id.or.oo.pr/files/usr/var/lib/proot-distro/installed-rootfs/alpine",
            "/data/data/id.or.oo.pr/files/usr/share/proot-distro/plugins/alpine.sh",
            "aarch64",
        );
        assert_eq!(converted.name, "Alpine Linux");
        assert_eq!(converted.source_ref(), "alpine");
    }

    #[test]
    fn oci_descriptor_exposes_common_fields() {
        let descriptor = InstallDescriptor::oci_image(
            "Debian",
            "/data/data/id.or.oo.pr/files/usr/var/lib/proot-distro/containers/debian/rootfs",
            "/data/data/id.or.oo.pr/files/usr/var/lib/proot-distro/containers/debian/manifest.json",
            "docker.io/library/debian:stable",
            Some("arm64".to_string()),
        );

        assert_eq!(descriptor.name, "Debian");
        assert_eq!(descriptor.kind(), InstallSourceKind::OciImage);
        assert_eq!(descriptor.source_ref(), "docker.io/library/debian:stable");
        assert_eq!(descriptor.architecture(), Some("arm64"));
        assert_eq!(
            descriptor.metadata_path(),
            Some(Path::new(
                "/data/data/id.or.oo.pr/files/usr/var/lib/proot-distro/containers/debian/manifest.json"
            ))
        );
    }

    #[test]
    fn round_trips_oci_metadata_json() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("pr-cli-oci-metadata-{}-{}", std::process::id(), nanos));
        let metadata_path = base.join("manifest.json");
        let metadata = OciInstallMetadata::new(
            "debian",
            "docker.io/library/debian:stable",
            "registry-1.docker.io/library/debian:stable",
            "aarch64",
            1_717_176_400,
        );

        write_oci_install_metadata(&metadata_path, &metadata).expect("write metadata");
        let loaded = load_oci_install_metadata(&metadata_path).expect("load metadata");
        assert_eq!(loaded, metadata);

        let _ = std::fs::remove_dir_all(base);
    }
}
