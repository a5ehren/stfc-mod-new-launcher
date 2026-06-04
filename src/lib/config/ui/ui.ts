import type { ConfigDefinition } from "../structure";

const experimental_feature = "[EXPERIMENTAL] ";

export const uiDefinitions: ConfigDefinition[] = [
	{
		group: "UI",
		key: "disable_escape_exit",
		label: "Disable Escape Exit",
		type: "checkbox",
		description: 'Prevent "escape" from prompting to exit the game',
	},
	{
		group: "UI",
		key: "disable_first_popup",
		label: "Disable First Popup",
		type: "checkbox",
		description: `${experimental_feature}Prevent the first popup advert from appearing`,
	},
	{
		group: "UI",
		key: "always_skip_reveal_sequence",
		label: "Always Skip Reveal Sequence",
		type: "checkbox",
		description: "Skip the reveal box animation, and just open the boxes",
	},
	{
		group: "UI",
		key: "disable_move_keys",
		label: "Disable Movement Keys",
		type: "checkbox",
		description: `${experimental_feature}Prevent the standard move keys from working`,
	},
	{
		group: "UI",
		key: "disable_toast_banners",
		label: "Disable Toast Banners",
		type: "checkbox",
		description: `${experimental_feature}Prevent the standard toast banners from working`,
	},
	{
		group: "UI",
		subgroup: "Chat",
		key: "disable_galaxy_chat",
		label: "Disable Galaxy Chat",
		type: "checkbox",
		description: "Change to true to remove Galaxy Chat from the game",
	},
	{
		group: "UI",
		subgroup: "Chat",
		key: "disable_veil_chat",
		label: "Disable Veil Chat",
		type: "checkbox",
		description: "Change to true to remove Veil Chat from the game",
	},
	{
		group: "UI",
		subgroup: "Donations",
		key: "extend_donation_max",
		label: "Extend Donation Max",
		type: "number",
		description: "Extend donation slider to max value for next Alliance level",
	},
	{
		group: "UI",
		subgroup: "Donations",
		key: "extend_donation_slider",
		label: "Extend Donation Slider",
		type: "checkbox",
		description: "Extend donation slider enable",
	},
	{
		group: "UI",
		subgroup: "Previews",
		key: "disable_preview_locate",
		label: "Disable Preview Locate",
		type: "checkbox",
		description: "Stop locate working when ship or node previews are displayed",
	},
	{
		group: "UI",
		subgroup: "Previews",
		key: "disable_preview_recall",
		label: "Disable Preview Recall",
		type: "checkbox",
		description: "Stop recall working when ship or node previews are displayed",
	},
	{
		group: "UI",
		subgroup: "Banners",
		key: "disabled_banner_types",
		label: "Disabled Banner Types",
		type: "banner",
		description:
			"Specifies types of banners you don't want to see (comma separated)",
	},
	{
		group: "UI",
		subgroup: "Previews",
		key: "show_armada_cargo",
		label: "Show Armada Cargo",
		type: "checkbox",
		description:
			"Set to true to always show the rewards for selected ships/stations",
	},
	{
		group: "UI",
		subgroup: "Previews",
		key: "show_cargo_default",
		label: "Show Default Cargo",
		type: "checkbox",
		description:
			"Set to true to always show the rewards for selected ships/stations",
	},
	{
		group: "UI",
		subgroup: "Previews",
		key: "show_hostile_cargo",
		label: "Show Hostile Cargo",
		type: "checkbox",
		description:
			"Set to true to always show the rewards for selected ships/stations",
	},
	{
		group: "UI",
		subgroup: "Previews",
		key: "show_player_cargo",
		label: "Show Player Cargo",
		type: "checkbox",
		description:
			"Set to true to always show the rewards for selected ships/stations",
	},
	{
		group: "UI",
		subgroup: "Previews",
		key: "show_station_cargo",
		label: "Show Station Cargo",
		type: "checkbox",
		description:
			"Set to true to always show the rewards for selected ships/stations",
	},
	{
		group: "UI",
		subgroup: "Unsupported",
		key: "auto_confirm_discovery",
		label: "Auto Confirm Discovery",
		type: "checkbox",
		description: "This setting is not used",
	},
];
