use serde::{Deserialize, Serialize};

use super::operation::{PlanStep, RequiredAction, StepId};
use super::readiness::WorkflowKind;

/// Bump whenever the disclaimer text changes; stale consent is rejected.
pub const DISCLAIMER_VERSION: &str = "2026-09-06.1";
pub const CONSENT_VALIDITY_MS: u64 = 30 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RiskId {
    Bootloop,
    DataLossOnRecovery,
    InterruptionHazard,
    SecurityPosture,
    UntetherStability,
    CommunityTooling,
    WarrantyAndSupport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RiskSeverity {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Risk {
    pub id: RiskId,
    pub severity: RiskSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PrerequisiteId {
    BatteryAbove50,
    WorkingButtons,
    BackupCompleted,
    StableCable,
    ComputerAwake,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Method {
    Ramdisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Exploit {
    Limera1n,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DataLoss {
    None,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tether {
    Untethered,
    SemiTethered,
    Tethered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingRequirements {
    pub risks: u32,
    pub disclaimer: u32,
}

/// Everything the user must see and agree to before a workflow may run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparationPlan {
    pub id: String,
    pub device_id: String,
    pub workflow: WorkflowKind,
    pub method: Method,
    pub exploit: Exploit,
    pub target_os_version: String,
    pub data_loss: DataLoss,
    pub tether: Tether,
    pub prerequisites: Vec<PrerequisiteId>,
    pub risks: Vec<Risk>,
    pub disclaimer_version: String,
    pub minimum_reading_seconds: ReadingRequirements,
    pub steps: Vec<PlanStep>,
}

impl PreparationPlan {
    /// Ramdisk-method jailbreak for A4 devices, mirroring the Legacy iOS Kit
    /// "Jailbreak Device" flow (limera1n → SSH ramdisk → untether).
    pub fn ramdisk_jailbreak(id: String, device_id: String, os_version: &str) -> Self {
        let step = |id, cancellable, point_of_no_return, estimated_seconds| PlanStep {
            id,
            cancellable,
            point_of_no_return,
            estimated_seconds,
            requires_action: None,
        };
        Self {
            id,
            device_id,
            workflow: WorkflowKind::Jailbreak,
            method: Method::Ramdisk,
            exploit: Exploit::Limera1n,
            target_os_version: os_version.to_owned(),
            data_loss: DataLoss::None,
            tether: Tether::Untethered,
            prerequisites: vec![
                PrerequisiteId::BatteryAbove50,
                PrerequisiteId::WorkingButtons,
                PrerequisiteId::BackupCompleted,
                PrerequisiteId::StableCable,
                PrerequisiteId::ComputerAwake,
            ],
            risks: vec![
                Risk {
                    id: RiskId::Bootloop,
                    severity: RiskSeverity::High,
                },
                Risk {
                    id: RiskId::DataLossOnRecovery,
                    severity: RiskSeverity::High,
                },
                Risk {
                    id: RiskId::InterruptionHazard,
                    severity: RiskSeverity::High,
                },
                Risk {
                    id: RiskId::SecurityPosture,
                    severity: RiskSeverity::Medium,
                },
                Risk {
                    id: RiskId::CommunityTooling,
                    severity: RiskSeverity::Medium,
                },
                Risk {
                    id: RiskId::UntetherStability,
                    severity: RiskSeverity::Low,
                },
                Risk {
                    id: RiskId::WarrantyAndSupport,
                    severity: RiskSeverity::Low,
                },
            ],
            disclaimer_version: DISCLAIMER_VERSION.to_owned(),
            minimum_reading_seconds: ReadingRequirements {
                risks: 5,
                disclaimer: 5,
            },
            steps: vec![
                PlanStep {
                    requires_action: Some(RequiredAction::EnterDfu),
                    ..step(StepId::EnterDfu, true, false, 60)
                },
                step(StepId::ExploitBootrom, true, false, 10),
                step(StepId::FetchResources, true, false, 25),
                step(StepId::BuildRamdisk, true, false, 15),
                step(StepId::BootRamdisk, false, false, 15),
                step(StepId::MountFilesystem, false, false, 8),
                step(StepId::InstallUntether, false, true, 40),
                step(StepId::RebootDevice, false, false, 20),
                step(StepId::VerifyJailbreak, false, false, 20),
            ],
        }
    }
}

/// The user's explicit, plan-bound authorization. Timestamps are epoch millis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentRecord {
    pub plan_id: String,
    pub prerequisites_confirmed: Vec<PrerequisiteId>,
    pub acknowledged_risk_ids: Vec<RiskId>,
    pub risk_reading_seconds: u32,
    pub risks_acknowledged_at: u64,
    pub disclaimer_version: String,
    pub disclaimer_reading_seconds: u32,
    pub disclaimer_accepted_at: u64,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConsentError {
    #[error("consent timestamps are invalid or expired")]
    InvalidTime,
    #[error("consent was recorded for a different plan")]
    PlanMismatch,
    #[error("prerequisite {0:?} was not confirmed")]
    MissingPrerequisite(PrerequisiteId),
    #[error("risk {0:?} was not acknowledged")]
    MissingRisk(RiskId),
    #[error("disclaimer version does not match the plan")]
    DisclaimerVersionMismatch,
    #[error("reading time is below the plan's minimum")]
    ReadingTooShort,
}

pub fn validate_timed_consent(
    plan: &PreparationPlan,
    consent: &ConsentRecord,
    issued_at: u64,
    now: u64,
) -> Result<(), ConsentError> {
    validate_consent(plan, consent)?;
    if issued_at > now
        || now - issued_at >= CONSENT_VALIDITY_MS
        || consent.risks_acknowledged_at
            < issued_at.saturating_add(u64::from(plan.minimum_reading_seconds.risks) * 1000)
        || consent.disclaimer_accepted_at
            < consent
                .risks_acknowledged_at
                .saturating_add(u64::from(plan.minimum_reading_seconds.disclaimer) * 1000)
        || consent.disclaimer_accepted_at > now
    {
        return Err(ConsentError::InvalidTime);
    }
    Ok(())
}

/// A workflow may only start when consent binds to this exact plan and covers
/// every prerequisite, every risk and the current disclaimer.
pub fn validate_consent(
    plan: &PreparationPlan,
    consent: &ConsentRecord,
) -> Result<(), ConsentError> {
    if consent.plan_id != plan.id {
        return Err(ConsentError::PlanMismatch);
    }
    if let Some(missing) = plan
        .prerequisites
        .iter()
        .find(|id| !consent.prerequisites_confirmed.contains(id))
    {
        return Err(ConsentError::MissingPrerequisite(*missing));
    }
    if let Some(missing) = plan
        .risks
        .iter()
        .find(|risk| !consent.acknowledged_risk_ids.contains(&risk.id))
    {
        return Err(ConsentError::MissingRisk(missing.id));
    }
    if consent.disclaimer_version != plan.disclaimer_version {
        return Err(ConsentError::DisclaimerVersionMismatch);
    }
    if consent.risk_reading_seconds < plan.minimum_reading_seconds.risks
        || consent.disclaimer_reading_seconds < plan.minimum_reading_seconds.disclaimer
    {
        return Err(ConsentError::ReadingTooShort);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> PreparationPlan {
        PreparationPlan::ramdisk_jailbreak("plan-1".into(), "dev".into(), "6.1.6")
    }

    fn full_consent(plan: &PreparationPlan) -> ConsentRecord {
        ConsentRecord {
            plan_id: plan.id.clone(),
            prerequisites_confirmed: plan.prerequisites.clone(),
            acknowledged_risk_ids: plan.risks.iter().map(|risk| risk.id).collect(),
            risk_reading_seconds: plan.minimum_reading_seconds.risks,
            risks_acknowledged_at: 1,
            disclaimer_version: plan.disclaimer_version.clone(),
            disclaimer_reading_seconds: plan.minimum_reading_seconds.disclaimer,
            disclaimer_accepted_at: 2,
        }
    }

    #[test]
    fn complete_consent_is_accepted() {
        let plan = plan();
        assert_eq!(validate_consent(&plan, &full_consent(&plan)), Ok(()));
    }

    #[test]
    fn native_consent_rejects_future_expired_and_impossible_reading_times() {
        let plan = plan();
        let mut consent = full_consent(&plan);
        consent.risks_acknowledged_at = 6_000;
        consent.disclaimer_accepted_at = 11_000;
        assert_eq!(
            validate_timed_consent(&plan, &consent, 1_000, 11_000),
            Ok(())
        );
        for now in [10_999, 1_000 + CONSENT_VALIDITY_MS] {
            assert_eq!(
                validate_timed_consent(&plan, &consent, 1_000, now),
                Err(ConsentError::InvalidTime)
            );
        }
        consent.disclaimer_accepted_at = 10_000;
        assert_eq!(
            validate_timed_consent(&plan, &consent, 1_000, 11_000),
            Err(ConsentError::InvalidTime)
        );
        consent.risks_acknowledged_at = 0;
        assert_eq!(
            validate_timed_consent(&plan, &consent, 1_000, 11_000),
            Err(ConsentError::InvalidTime)
        );
    }

    #[test]
    fn consent_must_cover_all_presented_risks() {
        let plan = plan();
        let mut consent = full_consent(&plan);
        consent
            .acknowledged_risk_ids
            .retain(|id| *id != RiskId::SecurityPosture);
        assert_eq!(
            validate_consent(&plan, &consent),
            Err(ConsentError::MissingRisk(RiskId::SecurityPosture))
        );
    }

    #[test]
    fn stale_disclaimer_is_rejected() {
        let plan = plan();
        let mut consent = full_consent(&plan);
        consent.disclaimer_version = "2020-01-01".into();
        assert_eq!(
            validate_consent(&plan, &consent),
            Err(ConsentError::DisclaimerVersionMismatch)
        );
    }

    #[test]
    fn consent_is_bound_to_its_plan() {
        let plan = plan();
        let mut consent = full_consent(&plan);
        consent.plan_id = "plan-2".into();
        assert_eq!(
            validate_consent(&plan, &consent),
            Err(ConsentError::PlanMismatch)
        );
    }

    #[test]
    fn reading_time_must_meet_the_minimum() {
        let plan = plan();
        let mut consent = full_consent(&plan);
        consent.disclaimer_reading_seconds = 3;
        assert_eq!(
            validate_consent(&plan, &consent),
            Err(ConsentError::ReadingTooShort)
        );
    }

    #[test]
    fn plan_marks_the_write_step_as_point_of_no_return() {
        let plan = plan();
        let write = plan
            .steps
            .iter()
            .find(|step| step.id == StepId::InstallUntether)
            .expect("write step");
        assert!(write.point_of_no_return);
        assert!(!write.cancellable);
        assert_eq!(
            plan.steps[0].requires_action,
            Some(RequiredAction::EnterDfu)
        );
    }
}
