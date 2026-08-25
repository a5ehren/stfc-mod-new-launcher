import type { WizardPlanDto } from "@/types/launcher";

export type WizardStep = "warnings" | "relocate" | "instances" | "done";

export const MAX_INSTANCES = 8;

export interface WizardState {
	step: WizardStep;
	acknowledged: boolean;
	needsRelocation: boolean;
	existingNames: string[];
	count: number;
}

export function initialWizardState(plan: WizardPlanDto): WizardState {
	return {
		step: "warnings",
		acknowledged: false,
		needsRelocation: plan.needsRelocation,
		existingNames: plan.existingNames,
		count: 1,
	};
}

/** First `count` auto-generated names (alt2, alt3, …) not already in use. */
export function generatedNames(existing: string[], count: number): string[] {
	const names: string[] = [];
	for (let i = 2; names.length < count; i++) {
		const candidate = `alt${i}`;
		if (!existing.includes(candidate)) names.push(candidate);
	}
	return names;
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
