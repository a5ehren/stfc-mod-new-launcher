import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
	LauncherStatus,
	LauncherUpdateInfo,
	LegacyCleanupPlan,
	ModChannel,
	ProgressEvent,
} from "@/types/launcher";

export function getLauncherStatus(): Promise<LauncherStatus> {
	return invoke("get_launcher_status");
}

export function setModChannel(channel: ModChannel): Promise<LauncherStatus> {
	return invoke("set_mod_channel", { channel });
}

export function setGamePath(path: string): Promise<LauncherStatus> {
	return invoke("set_game_path", { path });
}

export function openLogs(): Promise<void> {
	return invoke("open_logs");
}

export function openRawConfig(): Promise<void> {
	return invoke("open_raw_config");
}

export function openConfigEditor(): Promise<void> {
	return invoke("open_config_editor");
}

export function readRawConfig(): Promise<string> {
	return invoke("read_raw_config");
}

export function saveRawConfig(text: string): Promise<void> {
	return invoke("save_raw_config", { text });
}

export function validateGamePath(
	path: string,
): Promise<LauncherStatus["game"]> {
	return invoke("validate_game_path", { path });
}

export function launchGame(): Promise<void> {
	return invoke("launch_game");
}

export function updateGame(): Promise<boolean> {
	return invoke("update_game");
}

export function updateMod(): Promise<void> {
	return invoke("update_mod");
}

export function checkLauncherUpdate(): Promise<LauncherUpdateInfo | null> {
	return invoke("check_launcher_update");
}

export function getWindowsLegacyCleanupPlan(
	gameRoot: string,
): Promise<LegacyCleanupPlan> {
	return invoke("get_windows_legacy_cleanup_plan", { gameRoot });
}

export function applyManagedMigration(
	gameRoot: string,
	removeStaleDll: boolean,
): Promise<void> {
	return invoke("apply_managed_migration", { gameRoot, removeStaleDll });
}

export function onProgress(
	callback: (event: ProgressEvent) => void,
): Promise<() => void> {
	return listen<ProgressEvent>("launcher://progress", (event) =>
		callback(event.payload),
	);
}
