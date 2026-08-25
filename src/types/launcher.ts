export type ModChannel = "stable" | "prerelease";
export type LaunchMode = "managed" | "windowsProxyDll";

export type GameStatus = {
	known: boolean;
	path: string | null;
	installedVersion: number | null;
	latestVersion: number | null;
	updateAvailable: boolean;
};

export type ModStatus = {
	installed: boolean;
	installedVersion: string | null;
	latestVersion: string | null;
	channel: ModChannel;
	updateAvailable: boolean;
	launchMode: LaunchMode;
};

export type LauncherStatus = {
	game: GameStatus;
	modStatus: ModStatus;
	launcherUpdateAvailable: boolean;
	multiInstance: MultiInstanceState;
};

export interface MultiInstanceState {
	enabled: boolean;
	sharedGameRoot: string | null;
	instances: Instance[];
}

export interface Instance {
	name: string;
	osUsername: string;
	createdAt: string;
	lastBackupAt: string | null;
}

export interface WizardPlanDto {
	needsRelocation: boolean;
	gameSource: string | null;
	sharedRoot: string;
}

export interface InstanceStatusDto {
	name: string;
	osUsername: string;
	running: boolean;
	pid: number | null;
	lastBackupAt: string | null;
}

export type ProgressEvent = {
	operation: string;
	phase: string;
	message: string;
	current: number | null;
	total: number | null;
};

export type LegacyCleanupPlan = {
	staleDll: string | null;
	filesToMove: LegacyFileMove[];
};

export type LegacyFileMove = {
	source: string;
	destinationKind: "config" | "log";
};

export type LauncherUpdateInfo = {
	version: string;
	body: string | null;
};
