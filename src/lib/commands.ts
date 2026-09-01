import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
	InstanceStatusDto,
	LauncherStatus,
	LauncherUpdateInfo,
	LegacyCleanupPlan,
	ModChannel,
	MultiInstanceState,
	ProgressEvent,
	WizardPlanDto,
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

// Dev-only: the command is only registered in debug builds.
export function openDevtools(): Promise<void> {
	return invoke("open_devtools");
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

export function checkGameUpdate(): Promise<LauncherStatus> {
	return invoke("check_game_update");
}

export function updateMod(): Promise<void> {
	return invoke("update_mod");
}

export function checkModUpdate(): Promise<LauncherStatus> {
	return invoke("check_mod_update");
}

export function checkLauncherUpdate(): Promise<LauncherUpdateInfo | null> {
	return invoke("check_launcher_update");
}

export function installLauncherUpdate(): Promise<boolean> {
	return invoke("install_launcher_update");
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

export function miWizardPlan(): Promise<WizardPlanDto> {
	return invoke("mi_wizard_plan");
}

export function miProvision(names: string[]): Promise<MultiInstanceState> {
	return invoke("mi_provision", { names });
}

export function miSetEnabled(enabled: boolean): Promise<void> {
	return invoke("mi_set_enabled", { enabled });
}

export function miStartInstance(name: string): Promise<number> {
	return invoke("mi_start_instance", { name });
}

export function miStopInstance(name: string): Promise<void> {
	return invoke("mi_stop_instance", { name });
}

export function miInstanceStatus(): Promise<InstanceStatusDto[]> {
	return invoke("mi_instance_status");
}

export function miBackupInstance(name: string): Promise<string> {
	return invoke("mi_backup_instance", { name });
}

export function miRestoreInstance(name: string): Promise<void> {
	return invoke("mi_restore_instance", { name });
}

export function miRemoveInstance(name: string, force: boolean): Promise<void> {
	return invoke("mi_remove_instance", { name, force });
}

export function miSetInstanceLabel(name: string, label: string): Promise<void> {
	return invoke("mi_set_instance_label", { name, label });
}
