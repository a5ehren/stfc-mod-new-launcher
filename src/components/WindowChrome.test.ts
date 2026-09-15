import { getCurrentWindow } from "@tauri-apps/api/window";
import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import WindowChrome from "./WindowChrome.vue";

const windowApi = {
	minimize: vi.fn(),
	toggleMaximize: vi.fn(),
	close: vi.fn(),
	startDragging: vi.fn(),
	startResizeDragging: vi.fn(),
};

vi.mock("@tauri-apps/api/window", () => ({
	getCurrentWindow: vi.fn(() => windowApi),
}));

describe("WindowChrome", () => {
	it("mounts without a Tauri runtime", () => {
		expect(() => mount(WindowChrome)).not.toThrow();
		expect(getCurrentWindow).not.toHaveBeenCalled();
	});

	it("wires the window control buttons", async () => {
		const wrapper = mount(WindowChrome);
		const buttons = wrapper.findAll(".window-controls button");

		await buttons[0]?.trigger("click");
		expect(windowApi.minimize).toHaveBeenCalled();
		await buttons[1]?.trigger("click");
		expect(windowApi.toggleMaximize).toHaveBeenCalled();
		await buttons[2]?.trigger("click");
		expect(windowApi.close).toHaveBeenCalled();
	});

	it("starts resize dragging from edges and corners", () => {
		const wrapper = mount(WindowChrome);

		// jsdom has no PointerEvent; MouseEvent with a pointerdown type works.
		wrapper
			.find(".resize-edge.bottom")
			.element.dispatchEvent(new MouseEvent("pointerdown", { button: 0 }));
		expect(windowApi.startResizeDragging).toHaveBeenCalledWith("South");

		wrapper
			.find(".resize-corner.top-left")
			.element.dispatchEvent(new MouseEvent("pointerdown", { button: 0 }));
		expect(windowApi.startResizeDragging).toHaveBeenCalledWith("NorthWest");
	});

	it("has no resize layer over the window controls", () => {
		const wrapper = mount(WindowChrome);
		expect(wrapper.find(".resize-corner.top-right").exists()).toBe(false);
	});
});
