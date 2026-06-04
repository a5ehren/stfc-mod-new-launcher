export type ModChannel = "stable" | "prerelease";
export type LaunchMode = "managed" | "windowsProxyDll";

export type GameStatus = {
	known: boolean;
	path: string | null;
	installedVersion: number | null;
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
};

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
