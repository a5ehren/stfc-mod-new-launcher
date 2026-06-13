import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import {
	clearBootstrapError,
	resetBootstrapStatus,
	setBootstrapError,
	setBootstrapStatus,
} from "@/lib/bootstrap-status";
import DevOverlay from "./DevOverlay.vue";

describe("DevOverlay", () => {
	it("shows the startup status by default", () => {
		resetBootstrapStatus();
		clearBootstrapError();

		const wrapper = mount(DevOverlay);

		expect(wrapper.text()).toContain("Starting launcher");
	});

	it("shows runtime errors when present", async () => {
		resetBootstrapStatus();
		clearBootstrapError();
		setBootstrapStatus("Booting UI");
		setBootstrapError(new Error("frontend exploded"));

		const wrapper = mount(DevOverlay);

		expect(wrapper.text()).toContain("Booting UI");
		expect(wrapper.text()).toContain("frontend exploded");
	});
});
