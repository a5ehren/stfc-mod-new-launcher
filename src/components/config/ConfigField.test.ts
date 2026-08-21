import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import ConfigField from "./ConfigField.vue";

describe("ConfigField", () => {
	it("renders empty text fields as empty strings", () => {
		const wrapper = mount(ConfigField, {
			props: {
				definition: {
					group: "Config",
					key: "settings_url",
					label: "Settings URL",
					type: "textbox",
					description: "Ignored for the test",
				},
				modelValue: true,
			},
		});

		expect(
			(wrapper.find("input[type='text']").element as HTMLInputElement).value,
		).toBe("");
	});

	it("renders dropdown fields with their options and current value", () => {
		const wrapper = mount(ConfigField, {
			props: {
				definition: {
					group: "UI",
					key: "hud_missions",
					label: "Missions",
					type: "dropdown",
					options: ["auto", "show", "hide"],
					description: "HUD visibility mode: auto, show, or hide",
				},
				modelValue: "hide",
			},
		});

		const select = wrapper.find("select");
		expect(select.exists()).toBe(true);
		expect((select.element as HTMLSelectElement).value).toBe("hide");
		expect(wrapper.findAll("option").map((option) => option.text())).toEqual([
			"auto",
			"show",
			"hide",
		]);
	});
});
