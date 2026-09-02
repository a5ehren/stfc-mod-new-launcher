import { open } from "@tauri-apps/plugin-dialog";
import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import ViewscreenButton from "@/components/briefing/ViewscreenButton.vue";
import {
	getLauncherStatus,
	launchGame,
	miInstanceStatus,
	setGamePath,
	updateGame,
	validateGamePath,
} from "@/lib/commands";
import MainLauncher from "./MainLauncher.vue";

vi.mock("@/lib/commands", () => ({
	getLauncherStatus: vi.fn(async () => ({
		game: {
			known: true,
			path: "/game",
			installedVersion: 168,
			latestVersion: 169,
			updateAvailable: true,
		},
		modStatus: {
			installed: true,
			installedVersion: "v1.0.0",
			latestVersion: "v1.1.0",
			channel: "stable",
			updateAvailable: true,
			launchMode: "managed",
		},
		launcherUpdateAvailable: false,
		multiInstance: { enabled: false, sharedGameRoot: null, instances: [] },
	})),
	setModChannel: vi.fn(),
	openLogs: vi.fn(),
	openRawConfig: vi.fn(),
	openConfigEditor: vi.fn(),
	launchGame: vi.fn(),
	setGamePath: vi.fn(),
	validateGamePath: vi.fn(),
	updateGame: vi.fn(),
	updateMod: vi.fn(),
	checkModUpdate: vi.fn(async () => {
		throw new Error("offline in tests");
	}),
	checkGameUpdate: vi.fn(async () => {
		throw new Error("offline in tests");
	}),
	checkLauncherUpdate: vi.fn(async () => null),
	installLauncherUpdate: vi.fn(async () => false),
	onProgress: vi.fn(async () => vi.fn()),
	miInstanceStatus: vi.fn(async () => []),
	miStartInstance: vi.fn(),
	miStopInstance: vi.fn(),
	miBackupInstance: vi.fn(),
	miRestoreInstance: vi.fn(),
	miRemoveInstance: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
	open: vi.fn(),
	confirm: vi.fn(async () => true),
}));

vi.mock("@tauri-apps/plugin-process", () => ({
	relaunch: vi.fn(),
}));

describe("MainLauncher", () => {
	it("renders permanent and conditional actions", async () => {
		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));
		const labels = wrapper
			.findAllComponents(ViewscreenButton)
			.map((button) => button.text());

		expect(wrapper.text()).toContain("Launch Game");
		expect(wrapper.text()).toContain("Open Raw Config");
		expect(wrapper.text()).toContain("Open Config Editor");
		expect(wrapper.text()).toContain("Open Logs");
		expect(wrapper.text()).toContain("Update Game");
		expect(wrapper.text()).toContain("Update Mod");
		expect(wrapper.text()).toContain("Stable");

		expect(wrapper.find(".data-cascade").exists()).toBe(false);
		expect(wrapper.find(".launch-status").exists()).toBe(true);
		expect(labels.slice(-5)).toEqual([
			"Open Config Editor",
			"Open Raw Config",
			"Open Logs",
			"Multi-Instance",
			"Launch Game",
		]);
	});

	it("renders the instances panel only when multi-instance mode is enabled", async () => {
		const enabledStatus = {
			game: {
				known: true,
				path: "/game",
				installedVersion: 168,
				latestVersion: 169,
				updateAvailable: false,
			},
			modStatus: {
				installed: true,
				installedVersion: "v1.0.0",
				latestVersion: "v1.1.0",
				channel: "stable",
				updateAvailable: false,
				launchMode: "managed",
			},
			launcherUpdateAvailable: false,
			multiInstance: {
				enabled: true,
				sharedGameRoot: "/Users/Shared/STFC/game",
				instances: [
					{
						name: "alt2",
						osUsername: "stfc-alt2",
						createdAt: "2026-08-24T00:00:00Z",
						lastBackupAt: null,
					},
				],
			},
		};
		vi.mocked(getLauncherStatus).mockResolvedValueOnce(enabledStatus as never);
		vi.mocked(miInstanceStatus).mockResolvedValueOnce([
			{
				name: "alt2",
				osUsername: "stfc-alt2",
				running: false,
				pid: null,
				lastBackupAt: null,
				label: null,
				isBase: false,
			},
		]);

		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(wrapper.find(".instance-panel").exists()).toBe(true);
		expect(wrapper.text()).toContain("alt2");

		vi.mocked(getLauncherStatus).mockResolvedValueOnce({
			...enabledStatus,
			multiInstance: { enabled: false, sharedGameRoot: null, instances: [] },
		} as never);
		const disabledWrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(disabledWrapper.find(".instance-panel").exists()).toBe(false);
	});

	it("hides update actions when no updates are available", async () => {
		vi.mocked(getLauncherStatus).mockResolvedValueOnce({
			game: {
				known: true,
				path: "/game",
				installedVersion: 168,
				latestVersion: 169,
				updateAvailable: false,
			},
			modStatus: {
				installed: true,
				installedVersion: "v1.0.0",
				latestVersion: "v1.1.0",
				channel: "stable",
				updateAvailable: false,
				launchMode: "managed",
			},
			launcherUpdateAvailable: false,
			multiInstance: { enabled: false, sharedGameRoot: null, instances: [] },
		});

		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(wrapper.text()).not.toContain("Update Game");
		expect(wrapper.text()).not.toContain("Update Mod");
	});

	it("surfaces launch errors in the status strip", async () => {
		vi.mocked(getLauncherStatus).mockResolvedValueOnce({
			game: {
				known: true,
				path: "/game",
				installedVersion: 168,
				latestVersion: 169,
				updateAvailable: false,
			},
			modStatus: {
				installed: true,
				installedVersion: "v1.0.0",
				latestVersion: "v1.1.0",
				channel: "stable",
				updateAvailable: false,
				launchMode: "managed",
			},
			launcherUpdateAvailable: false,
			multiInstance: { enabled: false, sharedGameRoot: null, instances: [] },
		});
		vi.mocked(launchGame).mockRejectedValueOnce(new Error("missing dylib"));

		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));
		const buttons = wrapper.findAllComponents(ViewscreenButton);
		await buttons[buttons.length - 1]?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(wrapper.text()).toContain("Launch failed: Error: missing dylib");
	});

	it("formats object-shaped launch errors", async () => {
		vi.mocked(getLauncherStatus).mockResolvedValueOnce({
			game: {
				known: true,
				path: "/game",
				installedVersion: 168,
				latestVersion: 169,
				updateAvailable: false,
			},
			modStatus: {
				installed: true,
				installedVersion: "v1.0.0",
				latestVersion: "v1.1.0",
				channel: "stable",
				updateAvailable: false,
				launchMode: "managed",
			},
			launcherUpdateAvailable: false,
			multiInstance: { enabled: false, sharedGameRoot: null, instances: [] },
		});
		vi.mocked(launchGame).mockRejectedValueOnce({
			kind: "invalidData",
			message: "game path is not known",
		});

		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));
		const buttons = wrapper.findAllComponents(ViewscreenButton);
		await buttons[buttons.length - 1]?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(wrapper.text()).toContain(
			"Launch failed: invalidData: game path is not known",
		);
	});

	it("does not overwrite failed game updates with a success message", async () => {
		vi.mocked(updateGame).mockRejectedValueOnce(new Error("network down"));

		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));
		await wrapper
			.findAllComponents(ViewscreenButton)
			.find((button) => button.text() === "Update Game")
			?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(wrapper.text()).toContain("Update failed: Error: network down");
		expect(wrapper.text()).not.toContain("Game update started");
	});

	it("prompts for a game folder when launch reports an unknown path", async () => {
		vi.mocked(getLauncherStatus).mockResolvedValueOnce({
			game: {
				known: false,
				path: null,
				installedVersion: null,
				latestVersion: null,
				updateAvailable: false,
			},
			modStatus: {
				installed: true,
				installedVersion: "v1.0.0",
				latestVersion: "v1.1.0",
				channel: "stable",
				updateAvailable: false,
				launchMode: "managed",
			},
			launcherUpdateAvailable: false,
			multiInstance: { enabled: false, sharedGameRoot: null, instances: [] },
		});
		vi.mocked(launchGame)
			.mockRejectedValueOnce({
				kind: "gamePath",
				message: "game path is not known",
			})
			.mockResolvedValueOnce(undefined);
		vi.mocked(open).mockResolvedValueOnce("/game");
		vi.mocked(validateGamePath).mockResolvedValueOnce({
			known: true,
			path: "/game",
			installedVersion: 168,
			latestVersion: 169,
			updateAvailable: false,
		});
		vi.mocked(setGamePath).mockResolvedValueOnce({
			game: {
				known: true,
				path: "/game",
				installedVersion: 168,
				latestVersion: 169,
				updateAvailable: false,
			},
			modStatus: {
				installed: true,
				installedVersion: "v1.0.0",
				latestVersion: "v1.1.0",
				channel: "stable",
				updateAvailable: false,
				launchMode: "managed",
			},
			launcherUpdateAvailable: false,
			multiInstance: { enabled: false, sharedGameRoot: null, instances: [] },
		});

		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));
		const buttons = wrapper.findAllComponents(ViewscreenButton);
		await buttons[buttons.length - 1]?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(open).toHaveBeenCalled();
		expect(setGamePath).toHaveBeenCalledWith("/game");
		expect(wrapper.text()).toContain("Game launch started");
	});

	it("prompts for a game folder when update reports an unknown path", async () => {
		vi.mocked(getLauncherStatus).mockResolvedValueOnce({
			game: {
				known: false,
				path: null,
				installedVersion: null,
				latestVersion: null,
				updateAvailable: true,
			},
			modStatus: {
				installed: true,
				installedVersion: "v1.0.0",
				latestVersion: "v1.1.0",
				channel: "stable",
				updateAvailable: false,
				launchMode: "managed",
			},
			launcherUpdateAvailable: false,
			multiInstance: { enabled: false, sharedGameRoot: null, instances: [] },
		});
		vi.mocked(updateGame)
			.mockRejectedValueOnce({
				kind: "gamePath",
				message: "game path is not known",
			})
			.mockResolvedValueOnce(true);
		vi.mocked(open).mockResolvedValueOnce("/game");
		vi.mocked(validateGamePath).mockResolvedValueOnce({
			known: true,
			path: "/game",
			installedVersion: 168,
			latestVersion: 169,
			updateAvailable: false,
		});
		vi.mocked(setGamePath).mockResolvedValueOnce({
			game: {
				known: true,
				path: "/game",
				installedVersion: 168,
				latestVersion: 169,
				updateAvailable: false,
			},
			modStatus: {
				installed: true,
				installedVersion: "v1.0.0",
				latestVersion: "v1.1.0",
				channel: "stable",
				updateAvailable: false,
				launchMode: "managed",
			},
			launcherUpdateAvailable: false,
			multiInstance: { enabled: false, sharedGameRoot: null, instances: [] },
		});

		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));
		const buttons = wrapper.findAllComponents(ViewscreenButton);
		await buttons
			.find((button) => button.text() === "Update Game")
			?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(open).toHaveBeenCalled();
		expect(setGamePath).toHaveBeenCalledWith("/game");
		expect(wrapper.text()).toContain("Game update complete");
	});
});
