import type { SyncTargetConfiguration, TomlConfig } from "./types";

export const groupDisplayNames: Record<string, string> = {
	Buffs: "Buffs",
	Config: "Config",
	Control: "Control",
	Display: "Graphics",
	UI: "User Interface",
	Shortcuts: "Hotkeys / Shortcuts",
	Sync: "Data Sync",
};

export type ConfigDefinition = {
	key: keyof TomlConfig | string;
	group?: string;
	subgroup?: string;
	label: string;
	type: "checkbox" | "slider" | "number" | "textbox" | "banner";
	min?: number;
	max?: number;
	step?: number;
	description?: string; // <-- new
	isGenerated?: boolean; // <-- for missing definitions
	isHidden?: boolean;
};

export type EditableTarget = { key: string; config: SyncTargetConfiguration };
