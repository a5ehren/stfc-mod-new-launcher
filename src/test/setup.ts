import { vi } from "vitest";

Object.defineProperty(window, "__TAURI_INTERNALS__", {
	value: {},
	configurable: true,
});

vi.mock("@tauri-apps/api/core", () => ({
	invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
	listen: vi.fn(async () => vi.fn()),
}));

vi.mock("@tauri-apps/api/window", () => ({
	getCurrentWindow: vi.fn(() => ({ label: "main" })),
	WebviewWindow: vi.fn(),
}));
