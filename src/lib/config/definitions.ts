import { defaultConfig } from "./defaults";
import type { ConfigDefinition } from "./structure";
import type { TomlConfig } from "./types";
import { buffsDefinitions } from "./ui/buffs.ts";
import { configDefinitions } from "./ui/config.ts";
import { controlDefinitions } from "./ui/control.ts";
import { graphicsDefinitions } from "./ui/graphics.ts";
import { patchesDefinitions } from "./ui/patches.ts";
import { shortcutDefinitions } from "./ui/shortcuts.ts";
import { syncDefinitions } from "./ui/sync.ts";
import { uiDefinitions } from "./ui/ui.ts";

export type { ConfigDefinition } from "./structure";

/** --- Combine all manual definitions --- */
export const manualDefinitions: ConfigDefinition[] = [
	...buffsDefinitions,
	...configDefinitions,
	...controlDefinitions,
	...graphicsDefinitions,
	...shortcutDefinitions,
	...uiDefinitions,
	...patchesDefinitions,
	...syncDefinitions,
];

export const allDefinitions: ConfigDefinition[] =
	generateFullDefinitions(defaultConfig);

export function generateFullDefinitions(
	defaultConfig: TomlConfig,
): ConfigDefinition[] {
	const definitions: ConfigDefinition[] = [...manualDefinitions];

	function generateNestedDefs(
		obj: Record<string, unknown>,
		group = "",
	): ConfigDefinition[] {
		return Object.entries(obj).flatMap(([key, value]) => {
			const fullKey = group ? `${group}.${key}` : key;

			// Handle nested objects
			if (value && typeof value === "object" && !Array.isArray(value)) {
				// Special case for dynamic sync targets
				if (fullKey === "sync.targets") {
					return Object.entries(value as Record<string, unknown>).flatMap(
						([targetName, targetObj]) =>
							generateNestedDefs(
								targetObj as Record<string, unknown>,
								`sync.targets.${targetName}`,
							),
					);
				}

				// Regular nested object
				return generateNestedDefs(value as Record<string, unknown>, fullKey);
			}

			return [
				{
					key: fullKey,
					group: group || "Definition Required",
					label: `${group ? `${group}.` : ""}${key}`
						.replace(/_/g, " ")
						.replace(/\b\w/g, (c) => c.toUpperCase()),
					type:
						typeof value === "boolean"
							? "checkbox"
							: typeof value === "number"
								? "number"
								: "textbox",
					description:
						"Generated definition for config key without manual metadata",
					isGenerated: value === undefined, // placeholders for top-level sync or missing target keys
				},
			];
		});
	}

	// Existing keys in manual definitions
	const existingKeys = definitions.map(
		(d) => `${d.group?.toLowerCase()}.${d.key}`,
	);

	// Generate missing definitions from default config
	const defaultItems = generateNestedDefs(defaultConfig);
	const missingItems = defaultItems.filter(
		(d) => !existingKeys.includes(d.key.toLowerCase()),
	);

	return [...definitions, ...missingItems];
}
