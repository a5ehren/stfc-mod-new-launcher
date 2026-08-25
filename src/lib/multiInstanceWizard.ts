import type { WizardPlanDto } from "@/types/launcher";

export type WizardStep = "warnings" | "relocate" | "instances" | "done";

export interface WizardState {
	step: WizardStep;
	acknowledged: boolean;
	needsRelocation: boolean;
	names: string[];
}

export const INSTANCE_NAME_PATTERN = /^[a-z0-9-]{1,16}$/;

export function initialWizardState(plan: WizardPlanDto): WizardState {
	return {
		step: "warnings",
		acknowledged: false,
		needsRelocation: plan.needsRelocation,
		names: [],
	};
}

export function nextStep(s: WizardState): WizardState {
	if (s.step === "warnings") {
		if (!s.acknowledged) return s;
		return { ...s, step: s.needsRelocation ? "relocate" : "instances" };
	}
	if (s.step === "relocate") return { ...s, step: "instances" };
	if (s.step === "instances") return { ...s, step: "done" };
	return s;
}
