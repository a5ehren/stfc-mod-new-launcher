import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import LcarsButton from "@/components/lcars/LcarsButton.vue";
import { getLauncherStatus } from "@/lib/commands";
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
	updateGame: vi.fn(),
	updateMod: vi.fn(),
	onProgress: vi.fn(async () => vi.fn()),
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
});
