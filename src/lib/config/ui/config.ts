import type { ConfigDefinition } from "../structure";

export const configDefinitions: ConfigDefinition[] = [
	{
		group: "Config",
		key: "assets_url_override",
		label: "Assets URL Override",
		type: "textbox",
		description: "Ignore this section, developer stuff only",
	},
	{
		group: "Config",
		key: "settings_url",
		label: "Settings URL",
		type: "textbox",
		description: "Ignore this section, developer stuff only",
	},
];
