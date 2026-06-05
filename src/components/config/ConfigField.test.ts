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
});
