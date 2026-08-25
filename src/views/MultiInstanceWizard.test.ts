import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { miProvision, miWizardPlan } from "@/lib/commands";
import MultiInstanceWizard from "./MultiInstanceWizard.vue";

vi.mock("@/lib/commands", () => ({
	miWizardPlan: vi.fn(async () => ({
		needsRelocation: false,
		gameSource: "/game",
		sharedRoot: "/Users/Shared/STFC/game",
		existingNames: [],
	})),
	miProvision: vi.fn(async () => ({})),
	onProgress: vi.fn(async () => vi.fn()),
}));

async function mountWizard() {
	const wrapper = mount(MultiInstanceWizard);
	await new Promise((resolve) => setTimeout(resolve, 0));
	return wrapper;
}

describe("MultiInstanceWizard", () => {
	beforeEach(() => {
		vi.mocked(miWizardPlan).mockClear();
		vi.mocked(miProvision).mockReset();
		vi.mocked(miProvision).mockResolvedValue({} as never);
	});

	it("cancel on a non-terminal step emits done without provisioning", async () => {
		const wrapper = await mountWizard();
		expect(wrapper.text()).toContain("Before you begin");

		await wrapper
			.findAll("button")
			.find((b) => b.text() === "Cancel")
			?.trigger("click");

		expect(wrapper.emitted("done")).toHaveLength(1);
		expect(miProvision).not.toHaveBeenCalled();
	});

	it("gates the warnings step on acknowledgment", async () => {
		const wrapper = await mountWizard();
		expect(wrapper.text()).toContain("Before you begin");

		const continueButton = wrapper
			.findAll("button")
			.find((b) => b.text() === "Continue");
		expect(continueButton?.attributes("disabled")).toBeDefined();

		await wrapper.find('input[type="checkbox"]').setValue(true);
		await continueButton?.trigger("click");
		// needsRelocation: false → skips straight to instances
		expect(wrapper.text()).toContain("Create instances");
	});

	it("shows relocation step when the game must move", async () => {
		vi.mocked(miWizardPlan).mockResolvedValueOnce({
			needsRelocation: true,
			gameSource: "/old/path",
			sharedRoot: "/Users/Shared/STFC/game",
			existingNames: [],
		});
		const wrapper = await mountWizard();
		await wrapper.find('input[type="checkbox"]').setValue(true);
		await wrapper
			.findAll("button")
			.find((b) => b.text() === "Continue")
			?.trigger("click");
		expect(wrapper.text()).toContain("/old/path");
		expect(wrapper.text()).toContain("/Users/Shared/STFC/game");
	});

	it("provisions the chosen number of auto-named instances", async () => {
		const wrapper = await mountWizard();
		await wrapper.find('input[type="checkbox"]').setValue(true);
		await wrapper
			.findAll("button")
			.find((b) => b.text() === "Continue")
			?.trigger("click");

		await wrapper.find(".count-input").setValue(2);
		expect(wrapper.text()).toContain("stfc-alt2");
		expect(wrapper.text()).toContain("stfc-alt3");

		await wrapper
			.findAll("button")
			.find((b) => b.text() === "Provision")
			?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(miProvision).toHaveBeenCalledWith(["alt2", "alt3"]);
		expect(wrapper.text()).toContain("Multi-instance mode enabled");

		await wrapper
			.findAll("button")
			.find((b) => b.text() === "Done")
			?.trigger("click");
		expect(wrapper.emitted("done")).toHaveLength(1);
	});

	it("surfaces provision errors and stays on the instances step", async () => {
		vi.mocked(miProvision).mockRejectedValueOnce({
			kind: "invalidData",
			message: "boom",
		});
		const wrapper = await mountWizard();
		await wrapper.find('input[type="checkbox"]').setValue(true);
		await wrapper
			.findAll("button")
			.find((b) => b.text() === "Continue")
			?.trigger("click");
		await wrapper
			.findAll("button")
			.find((b) => b.text() === "Provision")
			?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(wrapper.text()).toContain("boom");
		expect(wrapper.text()).toContain("Create instances");
	});
});
