import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
	applyManagedMigration,
	checkLauncherUpdate,
	getLauncherStatus,
	getWindowsLegacyCleanupPlan,
	launchGame,
	onProgress,
	openLogs,
	openRawConfig,
	readRawConfig,
	saveRawConfig,
	setModChannel,
	validateGamePath,
} from "./commands";

describe("command wrappers", () => {
	beforeEach(() => {
		vi.mocked(invoke).mockReset();
		vi.mocked(listen).mockReset();
	});

	it("invokes launcher status command", async () => {
		vi.mocked(invoke).mockResolvedValue({ launcherUpdateAvailable: false });

		await getLauncherStatus();

		expect(invoke).toHaveBeenCalledWith("get_launcher_status");
	});

	it("invokes mod channel command with prerelease", async () => {
		vi.mocked(invoke).mockResolvedValue({});

		await setModChannel("prerelease");

		expect(invoke).toHaveBeenCalledWith("set_mod_channel", {
			channel: "prerelease",
		});
	});

	it("invokes open logs command", async () => {
		vi.mocked(invoke).mockResolvedValue(undefined);

		await openLogs();

		expect(invoke).toHaveBeenCalledWith("open_logs");
	});

	it("invokes open raw config command", async () => {
		vi.mocked(invoke).mockResolvedValue(undefined);

		await openRawConfig();

		expect(invoke).toHaveBeenCalledWith("open_raw_config");
	});

	it("invokes read raw config command", async () => {
		vi.mocked(invoke).mockResolvedValue("[control]\nhotkeys_enabled = true\n");

		const result = await readRawConfig();

		expect(invoke).toHaveBeenCalledWith("read_raw_config");
		expect(result).toBe("[control]\nhotkeys_enabled = true\n");
	});

	it("invokes save raw config command with text", async () => {
		vi.mocked(invoke).mockResolvedValue(undefined);

		await saveRawConfig("test config");

		expect(invoke).toHaveBeenCalledWith("save_raw_config", {
			text: "test config",
		});
	});

	it("invokes validate game path command with path", async () => {
		vi.mocked(invoke).mockResolvedValue({ known: true });

		await validateGamePath("/path/to/game");

		expect(invoke).toHaveBeenCalledWith("validate_game_path", {
			path: "/path/to/game",
		});
	});

	it("invokes launch game command", async () => {
		vi.mocked(invoke).mockResolvedValue(undefined);

		await launchGame();

		expect(invoke).toHaveBeenCalledWith("launch_game");
	});

	it("invokes check launcher update command", async () => {
		vi.mocked(invoke).mockResolvedValue(null);

		await checkLauncherUpdate();

		expect(invoke).toHaveBeenCalledWith("check_launcher_update");
	});

	it("invokes get windows legacy cleanup plan command", async () => {
		vi.mocked(invoke).mockResolvedValue({ staleDll: null, filesToMove: [] });

		await getWindowsLegacyCleanupPlan("/game");

		expect(invoke).toHaveBeenCalledWith("get_windows_legacy_cleanup_plan", {
			gameRoot: "/game",
		});
	});

	it("invokes apply managed migration command", async () => {
		vi.mocked(invoke).mockResolvedValue(undefined);

		await applyManagedMigration("/game", true);

		expect(invoke).toHaveBeenCalledWith("apply_managed_migration", {
			gameRoot: "/game",
			removeStaleDll: true,
		});
	});

	it("invokes onProgress with event listener", async () => {
		const mockUnlisten = vi.fn();
		vi.mocked(listen).mockResolvedValue(mockUnlisten);

		const callback = vi.fn();
		const unlisten = await onProgress(callback);

		expect(listen).toHaveBeenCalledWith(
			"launcher://progress",
			expect.any(Function),
		);
		expect(unlisten).toBe(mockUnlisten);
	});
});
