import { describe, expect, it } from "vitest";
import { initialWizardState, nextStep } from "./multiInstanceWizard";

describe("multiInstanceWizard", () => {
	it("skips relocation step when game already at shared root", () => {
		const s = initialWizardState({
			needsRelocation: false,
			gameSource: "/Users/Shared/STFC/game",
			sharedRoot: "/Users/Shared/STFC/game",
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
		});
		const acked = { ...s, acknowledged: true };
		expect(nextStep(acked).step).toBe("relocate");
	});

	it("requires warning acknowledgment before advancing", () => {
		const s = initialWizardState({
			needsRelocation: false,
			gameSource: null,
			sharedRoot: "/x",
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
		});
		const atInstances = nextStep({ ...s, acknowledged: true });
		expect(nextStep(atInstances).step).toBe("done");
	});

	it("done is terminal", () => {
		const s = initialWizardState({
			needsRelocation: false,
			gameSource: null,
			sharedRoot: "/x",
		});
		const done = nextStep(nextStep({ ...s, acknowledged: true }));
		expect(done.step).toBe("done");
		expect(nextStep(done)).toEqual(done);
	});
});
