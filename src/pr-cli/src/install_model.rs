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

#[cfg(test)]
mod tests {
    use super::*;

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
}
