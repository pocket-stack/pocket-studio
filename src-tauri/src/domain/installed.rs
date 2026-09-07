use super::{catalog::InstalledPackage, store::Catalog};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeApplication {
    pub bundle_id: String,
    pub product_version: Option<String>,
    pub build_number: Option<String>,
    pub application_type: Option<String>,
    pub receipt_build_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationObservation {
    pub applications: Vec<NativeApplication>,
    pub observed_at: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstalledReadState {
    Fresh,
    Stale,
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSnapshot {
    pub device_id: String,
    pub entries: Vec<InstalledPackage>,
    pub state: InstalledReadState,
    pub observed_at: Option<u64>,
    pub issue: Option<String>,
}

pub fn map_installed(
    catalog: &Catalog,
    observation: &InstallationObservation,
) -> Vec<InstalledPackage> {
    observation
        .applications
        .iter()
        .filter_map(|native| {
            let app = catalog.apps.iter().find(|app| {
                catalog
                    .releases
                    .iter()
                    .filter(|release| release.app_id == app.id)
                    .any(|release| {
                        release
                            .artifacts
                            .iter()
                            .any(|artifact| artifact.native_identity.bundle_id == native.bundle_id)
                    })
            })?;
            let releases: Vec<_> = catalog
                .releases
                .iter()
                .filter(|release| release.app_id == app.id)
                .collect();
            let exact = releases.iter().find_map(|release| {
                release
                    .artifacts
                    .iter()
                    .find(|artifact| {
                        artifact.native_identity.bundle_id == native.bundle_id
                            && native.product_version.as_deref()
                                == Some(&artifact.native_identity.version)
                            && native.build_number.as_deref()
                                == Some(&artifact.native_identity.build_number)
                            && native.receipt_build_id.as_deref() == Some(&artifact.build_id)
                    })
                    .map(|artifact| (*release, artifact))
            });
            Some(InstalledPackage {
                package_id: app.id.clone(),
                version: native
                    .product_version
                    .clone()
                    .or_else(|| native.build_number.clone())
                    .unwrap_or_default(),
                installed_at: None,
                native: Some(native.clone()),
                release_id: exact.map(|(release, _)| release.id.clone()),
                artifact_id: exact.map(|(_, artifact)| artifact.id.clone()),
                revision: exact.map(|(release, _)| release.revision),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::store::{Listing, ReleaseStatus};
    fn fixture() -> Catalog {
        let value: serde_json::Value =
            serde_json::from_str(include_str!("../../../contracts/store-v1/fixture-v1.json"))
                .unwrap();
        serde_json::from_str(value["catalog_utf8"].as_str().unwrap()).unwrap()
    }
    #[test]
    fn native_identity_not_display_name_maps_side_loaded_and_withdrawn_apps() {
        let mut catalog = fixture();
        catalog.apps[0].listing = Listing::Unlisted;
        catalog.releases[0].status = ReleaseStatus::Yanked;
        let mut second = catalog.apps[0].clone();
        second.id = "dev.another.clock".into();
        catalog.apps.push(second);
        let mut release = catalog.releases[0].clone();
        release.id = "second-release".into();
        release.app_id = "dev.another.clock".into();
        release.artifacts[0].native_identity.bundle_id = "native.clock".into();
        catalog.releases.push(release);
        let notes = NativeApplication {
            bundle_id: "dev.example.notes.ios".into(),
            product_version: Some("1.0.0".into()),
            build_number: Some("1".into()),
            application_type: Some("User".into()),
            receipt_build_id: None,
        };
        let clock = NativeApplication {
            bundle_id: "native.clock".into(),
            receipt_build_id: Some("fixture-build-1".into()),
            ..notes.clone()
        };
        let unrelated = NativeApplication {
            bundle_id: "another.application".into(),
            ..notes.clone()
        };
        let observation = InstallationObservation {
            applications: vec![notes, clock, unrelated],
            observed_at: 123,
        };
        let mapped = map_installed(&catalog, &observation);
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0].package_id, "dev.example.notes");
        assert_eq!(mapped[0].revision, None);
        assert_eq!(mapped[0].installed_at, None);
        assert_eq!(mapped[1].package_id, "dev.another.clock");
        assert_eq!(mapped[1].revision, Some(1));
        let mut changed = observation.clone();
        changed.applications[1].build_number = Some("2".into());
        assert_eq!(map_installed(&catalog, &changed)[1].revision, None);
        assert!(
            map_installed(
                &catalog,
                &InstallationObservation {
                    applications: vec![],
                    observed_at: 456
                }
            )
            .is_empty()
        );
    }
}
