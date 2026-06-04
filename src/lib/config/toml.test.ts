import { describe, expect, it } from "vitest";
import { defaultConfig } from "./defaults";
import { allDefinitions } from "./definitions";
import { generateToml } from "./toml";

describe("config TOML generation", () => {
	it("ports field metadata from modconfig", () => {
		expect(
			allDefinitions.some((definition) => definition.key === "hotkeys_enabled"),
		).toBe(true);
		expect(
			allDefinitions.some((definition) => definition.group === "Graphics"),
		).toBe(true);
	});

	it("generates expected sections from defaults", () => {
		const toml = generateToml(defaultConfig, false, false, false, false);

		expect(toml).toContain("[control]");
		expect(toml).toContain("hotkeys_enabled = true");
		expect(toml).toContain("[graphics]");
		expect(toml).toContain("[sync]");
	});
});
