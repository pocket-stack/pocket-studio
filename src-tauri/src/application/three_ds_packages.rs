use super::packages::PackageError;
use crate::domain::{catalog::PackageCategory, now_millis, packages::*, store::*, three_ds::*};
use std::collections::BTreeMap;

/// Who may manage what through the connected host. The launcher manages every
/// application except itself: launchers and runtimes are prepared through the
/// setup flow and never enter the ordinary install or delete path. A standalone
/// runtime exposes a management channel for its own `.pocket` guest only.
fn check_host(
    details: &ThreeDsDetails,
    app_id: &str,
    action: PackageAction,
    delivery: RuntimeDelivery,
) -> Result<(), PackageError> {
    let host_itself = details.host_app_id == app_id;
    if details.launcher {
        return if host_itself {
            Err(PackageError::RuntimeRequired)
        } else {
            Ok(())
        };
    }
    if host_itself
        && delivery == RuntimeDelivery::Bundled
        && matches!(action, PackageAction::Update | PackageAction::Reinstall)
    {
        Ok(())
    } else {
        Err(PackageError::RuntimeRequired)
    }
}

pub fn plan(
    catalog: &VerifiedCatalog,
    request: &PackageRequest,
    observed: &PackageObservation,
) -> Result<PackagePlan, PackageError> {
    let app = catalog
        .catalog()
        .apps
        .iter()
        .find(|app| app.id == request.app_id)
        .ok_or(PackageError::InvalidAction)?;
    let details = observed
        .device
        .three_ds
        .as_ref()
        .ok_or(PackageError::RuntimeRequired)?;
    if app.category == PackageCategory::Runtime {
        return Err(PackageError::RuntimeRequired);
    }
    let previous = request
        .installation_id
        .as_ref()
        .and_then(|id| {
            observed
                .managed
                .iter()
                .find(|item| &item.installation_id == id && item.app_id == app.id && item.installed)
        })
        .cloned();
    if request.action != PackageAction::Install && previous.is_none() {
        return Err(PackageError::InvalidAction);
    }
    if previous.as_ref().is_some_and(|value| value.unavailable) {
        return Err(PackageError::VerificationUnavailable);
    }
    let delivery = previous
        .as_ref()
        .map(|value| value.delivery)
        .or(request.delivery)
        .ok_or(PackageError::InvalidAction)?;
    check_host(details, &app.id, request.action, delivery)?;
    let mut selection = None;
    if request.action != PackageAction::Uninstall {
        if catalog.expired_at(now_millis()) {
            return Err(PackageError::CatalogExpired);
        }
        if app.listing != Listing::Listed {
            return Err(PackageError::Withdrawn);
        }
        let mut releases: Vec<_> = catalog
            .catalog()
            .releases
            .iter()
            .filter(|r| r.app_id == app.id && r.status == ReleaseStatus::Published)
            .collect();
        releases.sort_by(|a, b| {
            semver::Version::parse(&b.version)
                .unwrap()
                .cmp(&semver::Version::parse(&a.version).unwrap())
                .then_with(|| b.revision.cmp(&a.revision))
        });
        // Which artifact formats may satisfy the request. An explicit format on
        // an existing instance either limits the update to the `.pocket` guest
        // or asks for the native host to be replaced; it never switches formats.
        let formats: Vec<&str> = match (delivery, previous.as_ref(), request.format.as_deref()) {
            (RuntimeDelivery::Shared, _, _) => vec!["pocket"],
            (RuntimeDelivery::Bundled, None, format) => vec![format.unwrap_or("cia")],
            (RuntimeDelivery::Bundled, Some(prior), None) => vec!["pocket", prior.format.as_str()],
            (RuntimeDelivery::Bundled, Some(_), Some("pocket")) => vec!["pocket"],
            (RuntimeDelivery::Bundled, Some(prior), Some(format)) if format == prior.format => {
                vec![prior.format.as_str()]
            }
            (RuntimeDelivery::Bundled, Some(_), Some(_)) => {
                return Err(PackageError::InvalidAction);
            }
        };
        'releases: for release in releases {
            let mut preparation = false;
            for format in &formats {
                for artifact in release.artifacts.iter().filter(|a| a.format == *format) {
                    for target in &artifact.targets {
                        if !target.runtime_deliveries.contains(&delivery) {
                            continue;
                        }
                        // A `.pocket` guest cannot create a standalone host; it
                        // only updates the one that already owns the container.
                        let standalone = if artifact.format == "pocket"
                            && delivery == RuntimeDelivery::Bundled
                        {
                            match previous.as_ref() {
                                Some(prior) => Some(prior),
                                None => continue,
                            }
                        } else {
                            None
                        };
                        let verdict = evaluate_target_for(
                            artifact,
                            target,
                            Some(&observed.device),
                            observed.facts,
                            standalone,
                        );
                        if verdict == StoreVerdict::RequiresPreparation {
                            preparation = true;
                            continue;
                        }
                        if verdict != StoreVerdict::Compatible {
                            continue;
                        }
                        selection = Some((release, artifact, target));
                        break 'releases;
                    }
                }
            }
            if preparation {
                return Err(PackageError::RuntimeRequired);
            }
        }
        if selection.is_none() {
            return Err(PackageError::Incompatible);
        }
    }
    let (format, native_identity) = if let Some((_, artifact, _)) = selection {
        (
            artifact.format.clone(),
            artifact
                .native_identity
                .clone()
                .or_else(|| previous.as_ref().and_then(|p| p.native_identity.clone())),
        )
    } else {
        let prior = previous.as_ref().unwrap();
        (prior.format.clone(), prior.native_identity.clone())
    };
    let id = if let Some(prior) = &previous {
        prior.installation_id.clone()
    } else {
        let container = if delivery == RuntimeDelivery::Shared {
            "launcher".into()
        } else {
            match &native_identity {
                Some(NativeIdentity::ThreeDsTitle { title_id, .. }) => format!("cia-{title_id}"),
                Some(NativeIdentity::HomebrewFile { .. }) => {
                    format!("3dsx-{}", &sha256_hex(app.id.as_bytes())[..16])
                }
                _ => return Err(PackageError::InvalidAction),
            }
        };
        installation_id(&container, &app.id)
    };
    let existing = observed.managed.iter().find(|p| p.installation_id == id);
    if request.action == PackageAction::Install && existing.is_some_and(|p| p.installed) {
        return Err(PackageError::InvalidAction);
    }
    if let (Some(prior), Some((release, artifact, _))) = (&previous, selection) {
        let old = semver::Version::parse(&prior.version)
            .map_err(|_| PackageError::VerificationUnavailable)?;
        let new = semver::Version::parse(&release.version).unwrap();
        if new < old || new == old && prior.revision.is_some_and(|r| release.revision < r) {
            return Err(PackageError::Downgrade);
        }
        // An unknown installed revision is never treated as older than the
        // catalog; the same version needs an explicit reinstall.
        if request.action == PackageAction::Update
            && (prior.matches_artifact(artifact) || new == old && prior.revision.is_none())
        {
            return Err(PackageError::InvalidAction);
        }
    }
    Ok(PackagePlan {
        id: uuid::Uuid::new_v4().to_string(),
        device_id: request.device_id.clone(),
        device_name: observed.device.marketing_name.clone(),
        app_id: app.id.clone(),
        names: app
            .locales
            .iter()
            .map(|(l, t)| (l.clone(), t.name.clone()))
            .collect::<BTreeMap<_, _>>(),
        action: request.action,
        installation: PackageInstallation::ThreeDs(Box::new(ThreeDsPackagePlan {
            installation_id: id,
            delivery,
            updates_host: format != "pocket",
            format,
            expected_generation: existing.map_or(0, |p| p.generation),
            previous,
            native_identity,
        })),
        delete_data: request.action == PackageAction::Uninstall
            && request.delete_data.unwrap_or(false),
        release_id: selection.map(|(r, _, _)| r.id.clone()),
        artifact: selection.map(|(_, a, _)| a.clone()),
        target: selection.map(|(_, _, t)| t.clone()),
        version: selection.map(|(r, _, _)| r.version.clone()),
        revision: selection.map(|(r, _, _)| r.revision),
        publication_id: catalog.catalog().publication_id.clone(),
        sequence: catalog.catalog().sequence,
        catalog_expires_at: catalog.catalog().expires_at,
        expires_at: now_millis() + 15 * 60 * 1000,
        steps: package_steps(request.action),
    })
}

/// Re-checks, right before a write, everything about the device that the plan
/// assumed: the connected host is still allowed to manage this application and
/// the installation instance has not moved on.
pub fn validate_state(
    plan: &PackagePlan,
    observed: &PackageObservation,
) -> Result<(), PackageError> {
    let managed = plan.managed().ok_or(PackageError::InvalidAction)?;
    let details = observed
        .device
        .three_ds
        .as_ref()
        .ok_or(PackageError::RuntimeRequired)?;
    check_host(details, &plan.app_id, plan.action, managed.delivery)?;
    let actual = observed
        .managed
        .iter()
        .find(|v| v.installation_id == managed.installation_id);
    if actual.is_some_and(|v| v.unavailable)
        || actual.map_or(0, |v| v.generation) != managed.expected_generation
    {
        return Err(PackageError::StateChanged);
    }
    if actual.is_some_and(|v| v.installed) != managed.previous.is_some() {
        return Err(PackageError::StateChanged);
    }
    Ok(())
}

/// Compares the device's own installation evidence with the plan after the
/// device reported the operation as verified. A record Studio cannot read is
/// never evidence either way.
pub fn verify(plan: &PackagePlan, observed: &PackageObservation) -> Result<(), PackageError> {
    let managed = plan.managed().ok_or(PackageError::InvalidAction)?;
    let record = observed
        .managed
        .iter()
        .find(|v| v.installation_id == managed.installation_id);
    if record.is_some_and(|v| v.unavailable) {
        return Err(PackageError::VerificationUnavailable);
    }
    if plan.action == PackageAction::Uninstall {
        return if record.is_some_and(|v| v.installed) {
            Err(PackageError::VerificationFailed)
        } else {
            Ok(())
        };
    }
    let actual = record
        .filter(|v| v.installed)
        .ok_or(PackageError::VerificationUnavailable)?;
    if !actual.matches_artifact(plan.artifact.as_ref().ok_or(PackageError::InvalidAction)?) {
        return Err(PackageError::VerificationFailed);
    }
    Ok(())
}
