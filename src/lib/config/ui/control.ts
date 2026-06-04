import type { ConfigDefinition } from "../structure";

export const controlDefinitions: ConfigDefinition[] = [
	{
		group: "Control",
		subgroup: "Options",
		key: "enable_experimental",
		label: "Enable Experimental Features",
		type: "checkbox",
		description: "Enable experimental features",
	},
	{
		group: "Control",
		subgroup: "Hotkeys",
		key: "hotkeys_enabled",
		label: "Hotkeys Enabled",
		type: "checkbox",
		description:
			"If you don't want any hotkeys you can set this to false (disables both mod and scopely hotkeys)",
	},
	{
		group: "Control",
		subgroup: "Hotkeys",
		key: "hotkeys_extended",
		label: "Extended Hotkeys",
		type: "checkbox",
		description:
			"If you prefer to disable the extended hotkeys set this to false",
	},
	{
		group: "Control",
		subgroup: "Hotkeys",
		key: "queue_enabled",
		label: "Queue Enabled",
		type: "checkbox",
		description:
			"If you have the Kir'Shara artifact, should the queue be enabled by default?",
	},
	{
		group: "Control",
		key: "select_timer",
		label: "Select Timer (ms)",
		type: "number",
		description:
			"Double-tap window in ms for 1-8 ship select/locate. Lower = faster, higher = slower. Default 500",
	},
	{
		group: "Control",
		subgroup: "Hotkeys",
		key: "use_scopely_hotkeys",
		label: "Use Scopely Hotkeys",
		type: "checkbox",
		description: "If you prefer to use Scopely's hotkeys set this to true",
	},
];
