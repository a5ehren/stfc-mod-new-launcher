import { describe, expect, it } from "vitest";
import {
	generatedNames,
	initialWizardState,
	nextStep,
} from "./multiInstanceWizard";

describe("multiInstanceWizard", () => {
	it("skips relocation step when game already at shared root", () => {
		const s = initialWizardState({
			needsRelocation: false,
			gameSource: "/Users/Shared/STFC/game",
			sharedRoot: "/Users/Shared/STFC/game",
			existingNames: [],
		});
		expect(s.step).toBe("warnings");
		// acknowledge warnings first (required before advancing)
		expect(nextStep({ ...s, acknowledged: true }).step).toBe("instances"); // no relocate step
	});

	it("includes relocation when needed", () => {
		const s = initialWizardState({
			needsRelocation: true,
			gameSource: "/old/path",
			sharedRoot: "/Users/Shared/STFC/game",
			existingNames: [],
		});
		const acked = { ...s, acknowledged: true };
		expect(nextStep(acked).step).toBe("relocate");
	});

	it("requires warning acknowledgment before advancing", () => {
		const s = initialWizardState({
			needsRelocation: false,
			gameSource: null,
			sharedRoot: "/x",
			existingNames: [],
		});
		expect(nextStep(s).step).not.toBe("done");
		const acked = { ...s, acknowledged: true };
		expect(nextStep(acked).step).toBe("instances");
	});

	it("instances step advances to done", () => {
		const s = initialWizardState({
			needsRelocation: false,
			gameSource: null,
			sharedRoot: "/x",
			existingNames: [],
		});
		const atInstances = nextStep({ ...s, acknowledged: true });
		expect(nextStep(atInstances).step).toBe("done");
	});

	it("generates sequential alt names, skipping existing ones", () => {
		expect(generatedNames([], 3)).toEqual(["alt2", "alt3", "alt4"]);
		expect(generatedNames(["alt2"], 2)).toEqual(["alt3", "alt4"]);
		expect(generatedNames([], 0)).toEqual([]);
	});

	it("done is terminal", () => {
		const s = initialWizardState({
			needsRelocation: false,
			gameSource: null,
			sharedRoot: "/x",
			existingNames: [],
		});
		const done = nextStep(nextStep({ ...s, acknowledged: true }));
		expect(done.step).toBe("done");
		expect(nextStep(done)).toEqual(done);
	});
});
