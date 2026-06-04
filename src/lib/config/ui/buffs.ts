import type { ConfigDefinition } from "../structure";

export const buffsDefinitions: ConfigDefinition[] = [
	{
		group: "Buffs",
		key: "use_out_of_dock_power",
		label: "Use Out of Dock Power",
		type: "checkbox",
		description:
			"This is the option to always show out of dock power for ships when they are docked. Defaults to false",
	},
];
