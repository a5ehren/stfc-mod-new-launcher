import { open } from "@tauri-apps/plugin-dialog";
import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import LcarsButton from "@/components/lcars/LcarsButton.vue";
import {
	getLauncherStatus,
	launchGame,
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
	onProgress: vi.fn(async () => vi.fn()),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
	open: vi.fn(),
}));

describe("MainLauncher", () => {
	it("renders permanent and conditional actions", async () => {
		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));
		const labels = wrapper
			.findAllComponents(LcarsButton)
			.map((button) => button.text());

		expect(wrapper.text()).toContain("Launch Game");
		expect(wrapper.text()).toContain("Open Raw Config");
		expect(wrapper.text()).toContain("Open Config Editor");
		expect(wrapper.text()).toContain("Open Logs");
		expect(wrapper.text()).toContain("Update Game");
		expect(wrapper.text()).toContain("Update Mod");
		expect(wrapper.text()).toContain("Stable");
		expect(wrapper.find(".lcars-shell").classes()).toContain("compact-header");
		expect(wrapper.find(".data-cascade").exists()).toBe(false);
		expect(wrapper.find(".launch-status").exists()).toBe(true);
		expect(labels.slice(-4)).toEqual([
			"Open Logs",
			"Update Game",
			"Update Mod",
			"Launch Game",
		]);
	});

	it("hides update actions when no updates are available", async () => {
		vi.mocked(getLauncherStatus).mockResolvedValueOnce({
			game: {
				known: true,
				path: "/game",
				installedVersion: 168,
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
		});
		vi.mocked(launchGame).mockRejectedValueOnce(new Error("missing dylib"));

		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));
		const buttons = wrapper.findAllComponents(LcarsButton);
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
		});
		vi.mocked(launchGame).mockRejectedValueOnce({
			kind: "invalidData",
			message: "game path is not known",
		});

		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));
		const buttons = wrapper.findAllComponents(LcarsButton);
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
			.findAllComponents(LcarsButton)
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
			updateAvailable: false,
		});
		vi.mocked(setGamePath).mockResolvedValueOnce({
			game: {
				known: true,
				path: "/game",
				installedVersion: 168,
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
		});

		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));
		const buttons = wrapper.findAllComponents(LcarsButton);
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
		});
		vi.mocked(updateGame)
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
			updateAvailable: false,
		});
		vi.mocked(setGamePath).mockResolvedValueOnce({
			game: {
				known: true,
				path: "/game",
				installedVersion: 168,
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
		});

		const wrapper = mount(MainLauncher);
		await new Promise((resolve) => setTimeout(resolve, 0));
		const buttons = wrapper.findAllComponents(LcarsButton);
		await buttons
			.find((button) => button.text() === "Update Game")
			?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(open).toHaveBeenCalled();
		expect(setGamePath).toHaveBeenCalledWith("/game");
		expect(wrapper.text()).toContain("Game update complete");
	});
});
