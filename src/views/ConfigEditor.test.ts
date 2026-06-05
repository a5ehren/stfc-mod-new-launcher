import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import LcarsButton from "@/components/lcars/LcarsButton.vue";
import ConfigEditor from "./ConfigEditor.vue";

vi.mock("@/lib/commands", () => ({
	readRawConfig: vi.fn(async () => ""),
	saveRawConfig: vi.fn(async () => undefined),
}));

describe("ConfigEditor", () => {
	it("starts with TOML preview collapsed and enables save when dirty", async () => {
		const wrapper = mount(ConfigEditor);
		await new Promise((resolve) => setTimeout(resolve, 0));
		const buttons = wrapper.findAllComponents(LcarsButton);
		const tabs = wrapper.findAll('[role="tab"]');
		const rows = wrapper.findAll(".tab-row");
		const fields = wrapper.findAll("label.config-field");

		expect(tabs.length).toBe(8);
		expect(rows).toHaveLength(2);
		expect(tabs[0].attributes("title")).toContain(
			"overall controller of the mod",
		);
		expect(tabs[0].attributes("aria-selected")).toBe("true");
		expect(wrapper.find(".lcars-shell").classes()).toContain("compact-header");
		expect(wrapper.find(".data-cascade").exists()).toBe(false);
		expect(wrapper.text()).toContain("Control Panel");
		expect(wrapper.text()).toContain("Show TOML Preview");
		expect(wrapper.text()).toContain("STFC Mod : Config");
		expect(wrapper.text()).toContain("Enable Experimental Features");
		expect(fields[0].attributes("title")).toContain(
			"Enable experimental features",
		);
		expect(wrapper.find("textarea").exists()).toBe(false);
		expect(buttons.map((button) => button.classes())).toEqual(
			expect.arrayContaining([expect.arrayContaining(["edge-single"])]),
		);

		await tabs[1].trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(wrapper.text()).toContain("UI Scale");
		expect(wrapper.find("output").text()).toBe("0.6");

		await wrapper.find("input[type='checkbox']").setValue(false);

		expect(wrapper.text()).toContain("Unsaved changes");
		expect(wrapper.find("button.save").attributes("disabled")).toBeUndefined();
	});
});
