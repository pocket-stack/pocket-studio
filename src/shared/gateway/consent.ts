import {
  GatewayError,
  type ConsentRecord,
  type PreparationPlan,
} from "./types";

/** The demo enforces the same consent boundary even when called outside a view. */
export function validateConsent(
  plan: PreparationPlan,
  consent: ConsentRecord,
  now: number,
): void {
  if (consent.planId !== plan.id)
    throw new GatewayError("unknownPlan", "Consent does not match this plan");
  if (
    plan.prerequisites.some(
      (id) => !consent.prerequisitesConfirmed.includes(id),
    ) ||
    plan.risks.some((risk) => !consent.acknowledgedRiskIds.includes(risk.id))
  ) {
    throw new GatewayError(
      "consentIncomplete",
      "Every prerequisite and risk must be acknowledged",
    );
  }
  if (consent.disclaimerVersion !== plan.disclaimerVersion)
    throw new GatewayError(
      "disclaimerOutdated",
      "Disclaimer version does not match",
    );
  if (
    !Number.isFinite(consent.riskReadingSeconds) ||
    !Number.isFinite(consent.disclaimerReadingSeconds) ||
    consent.riskReadingSeconds < plan.minimumReadingSeconds.risks ||
    consent.disclaimerReadingSeconds < plan.minimumReadingSeconds.disclaimer
  ) {
    throw new GatewayError(
      "readingTooShort",
      "Both reading durations must be satisfied",
    );
  }
  if (
    !Number.isFinite(consent.risksAcknowledgedAt) ||
    !Number.isFinite(consent.disclaimerAcceptedAt) ||
    consent.risksAcknowledgedAt <= 0 ||
    consent.disclaimerAcceptedAt - consent.risksAcknowledgedAt <
      plan.minimumReadingSeconds.disclaimer * 1000 ||
    consent.disclaimerAcceptedAt > now ||
    now - consent.disclaimerAcceptedAt > 30 * 60 * 1000
  ) {
    throw new GatewayError(
      "consentIncomplete",
      "Consent must be sequential, current, and dated in the past",
    );
  }
}
