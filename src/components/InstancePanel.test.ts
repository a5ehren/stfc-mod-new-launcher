import { confirm } from "@tauri-apps/plugin-dialog";
import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
	miInstanceStatus,
	miRemoveInstance,
	miStartInstance,
	miStopInstance,
} from "@/lib/commands";
import type { InstanceStatusDto } from "@/types/launcher";
import InstancePanel from "./InstancePanel.vue";

const ROWS: InstanceStatusDto[] = [
	{
		name: "alt2",
		osUsername: "stfc-alt2",
		running: true,
		pid: 1234,
		lastBackupAt: "2026-08-24T00:00:00Z",
	},
	{
		name: "alt3",
		osUsername: "stfc-alt3",
		running: false,
		pid: null,
		lastBackupAt: null,
	},
];

vi.mock("@/lib/commands", () => ({
	miInstanceStatus: vi.fn(async () => ROWS),
	miStartInstance: vi.fn(async () => 1234),
	miStopInstance: vi.fn(async () => {}),
	miBackupInstance: vi.fn(async () => "/backups/alt2/2026-08-24"),
	miRestoreInstance: vi.fn(async () => {}),
	miRemoveInstance: vi.fn(async () => {}),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
	confirm: vi.fn(async () => true),
}));

async function mountPanel() {
	const wrapper = mount(InstancePanel);
	await new Promise((resolve) => setTimeout(resolve, 0));
	return wrapper;
}

describe("InstancePanel", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it("shows one row per instance with running state", async () => {
		const wrapper = await mountPanel();

		expect(wrapper.text()).toContain("alt2");
		expect(wrapper.text()).toContain("alt3");
		expect(wrapper.text()).toContain("Running");
		expect(wrapper.text()).toContain("1234");
	});

	it("remove without backup asks for force confirm", async () => {
		const wrapper = await mountPanel();
		const alt3Row = wrapper
			.findAll(".instance-row")
			.find((row) => row.text().includes("alt3"));
		await alt3Row
			?.findAll("button")
			.find((button) => button.text() === "Remove")
			?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(confirm).toHaveBeenCalled();
		expect(vi.mocked(confirm).mock.calls[0]?.[0]).toMatch(/backup/i);
		expect(miRemoveInstance).toHaveBeenCalledWith("alt3", true);
	});

	it("remove with backup skips the confirm dialog", async () => {
		const wrapper = await mountPanel();
		const alt2Row = wrapper
			.findAll(".instance-row")
			.find((row) => row.text().includes("alt2"));
		await alt2Row
			?.findAll("button")
			.find((button) => button.text() === "Remove")
			?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(confirm).not.toHaveBeenCalled();
		expect(miRemoveInstance).toHaveBeenCalledWith("alt2", false);
	});

	it("declined confirm does not remove", async () => {
		vi.mocked(confirm).mockResolvedValueOnce(false);
		const wrapper = await mountPanel();
		const alt3Row = wrapper
			.findAll(".instance-row")
			.find((row) => row.text().includes("alt3"));
		await alt3Row
			?.findAll("button")
			.find((button) => button.text() === "Remove")
			?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(miRemoveInstance).not.toHaveBeenCalled();
	});

	it("start and stop are mutually exclusive per running state", async () => {
		const wrapper = await mountPanel();
		const rows = wrapper.findAll(".instance-row");
		const alt2 = rows.find((row) => row.text().includes("alt2"));
		const alt3 = rows.find((row) => row.text().includes("alt3"));

		const buttonByText = (
			row: (typeof rows)[number] | undefined,
			text: string,
		) => row?.findAll("button").find((button) => button.text() === text);

		// alt2 is running: Stop enabled, Start disabled
		expect(buttonByText(alt2, "Stop")?.attributes("disabled")).toBeUndefined();
		expect(buttonByText(alt2, "Start")?.attributes("disabled")).toBeDefined();
		// alt3 is stopped: Start enabled, Stop disabled
		expect(buttonByText(alt3, "Start")?.attributes("disabled")).toBeUndefined();
		expect(buttonByText(alt3, "Stop")?.attributes("disabled")).toBeDefined();

		await buttonByText(alt3, "Start")?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(miStartInstance).toHaveBeenCalledWith("alt3");

		await buttonByText(alt2, "Stop")?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(miStopInstance).toHaveBeenCalledWith("alt2");
	});

	it("surfaces command errors", async () => {
		vi.mocked(miStartInstance).mockRejectedValueOnce({
			kind: "invalidData",
			message: "already running",
		});
		const wrapper = await mountPanel();
		const alt3Row = wrapper
			.findAll(".instance-row")
			.find((row) => row.text().includes("alt3"));
		await alt3Row
			?.findAll("button")
			.find((button) => button.text() === "Start")
			?.trigger("click");
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(wrapper.text()).toContain("already running");
	});

	it("polls status on an interval and stops on unmount", async () => {
		vi.useFakeTimers();
		try {
			const wrapper = mount(InstancePanel);
			await vi.advanceTimersByTimeAsync(0);
			expect(miInstanceStatus).toHaveBeenCalledTimes(1);

			await vi.advanceTimersByTimeAsync(5000);
			expect(miInstanceStatus).toHaveBeenCalledTimes(2);

			wrapper.unmount();
			await vi.advanceTimersByTimeAsync(10000);
			expect(miInstanceStatus).toHaveBeenCalledTimes(2);
		} finally {
			vi.useRealTimers();
		}
	});
});
