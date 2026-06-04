import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import ConfigEditor from "./ConfigEditor.vue";

vi.mock("@/lib/commands", () => ({
	readRawConfig: vi.fn(async () => ""),
	saveRawConfig: vi.fn(async () => undefined),
}));

describe("ConfigEditor", () => {
	it("starts with TOML preview collapsed and enables save when dirty", async () => {
		const wrapper = mount(ConfigEditor);
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(wrapper.text()).toContain("Control Panel");
		expect(wrapper.text()).toContain("Show TOML Preview");
		expect(wrapper.find("textarea").exists()).toBe(false);

		await wrapper.find("input[type='checkbox']").setValue(false);

		expect(wrapper.text()).toContain("Unsaved changes");
		expect(wrapper.find("button.save").attributes("disabled")).toBeUndefined();
	});
});
