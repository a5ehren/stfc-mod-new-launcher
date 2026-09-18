import { afterEach, describe, expect, it, vi } from "vitest";
import { readRawConfig, saveRawConfig } from "@/lib/commands";
import { attachConfigBridge } from "@/lib/configBridge";

vi.mock("@/lib/commands", () => ({
	readRawConfig: vi.fn(async () => ""),
	saveRawConfig: vi.fn(),
}));

afterEach(() => {
	document.body.innerHTML = "";
});

function setup() {
	const frame = document.createElement("iframe");
	document.body.appendChild(frame);
	const onStatus = vi.fn();
	const detach = attachConfigBridge(frame, onStatus);
	return { frame, onStatus, detach };
}

function dispatchConfigMessage(
	frame: HTMLIFrameElement,
	data: unknown,
	origin = window.location.origin,
) {
	window.dispatchEvent(
		new MessageEvent("message", {
			data,
			origin,
			source: frame.contentWindow,
		}),
	);
}

describe("attachConfigBridge", () => {
	it("pushes TOML to the modconfig iframe when it reports ready", async () => {
		vi.mocked(readRawConfig).mockResolvedValueOnce("[ui]\nscale = 1\n");
		const { frame } = setup();
		const postMessage = vi.spyOn(frame.contentWindow as Window, "postMessage");

		dispatchConfigMessage(frame, { type: "modconfig-ready" });
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(readRawConfig).toHaveBeenCalled();
		expect(postMessage).toHaveBeenCalledWith(
			{ type: "stfc-launcher-config", toml: "[ui]\nscale = 1\n" },
			window.location.origin,
		);
	});

	it("saves TOML sent by the modconfig iframe and stops after detach", async () => {
		const { frame, onStatus, detach } = setup();

		dispatchConfigMessage(frame, {
			type: "modconfig-save",
			toml: "[a]\nb = 2\n",
		});
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(saveRawConfig).toHaveBeenCalledWith("[a]\nb = 2\n");
		expect(onStatus).toHaveBeenCalledWith("Mod configuration saved");

		detach();
		vi.mocked(saveRawConfig).mockClear();
		dispatchConfigMessage(frame, {
			type: "modconfig-save",
			toml: "[a]\nb = 3\n",
		});
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(saveRawConfig).not.toHaveBeenCalled();
	});

	it("surfaces bridge errors instead of rejecting silently", async () => {
		vi.mocked(readRawConfig).mockRejectedValueOnce(new Error("no config file"));
		const { frame, onStatus } = setup();

		dispatchConfigMessage(frame, { type: "modconfig-ready" });
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(onStatus).toHaveBeenCalledWith(
			expect.stringContaining(
				"Mod configuration failed: Error: no config file",
			),
		);
	});

	it("ignores messages with a mismatched origin, source, or malformed payload", async () => {
		const { frame } = setup();

		// Wrong origin
		dispatchConfigMessage(
			frame,
			{ type: "modconfig-save", toml: "[a]\n" },
			"https://evil.example",
		);

		// Wrong source (no source set; jsdom sets event.source to null)
		window.dispatchEvent(
			new MessageEvent("message", {
				data: { type: "modconfig-save", toml: "[b]\n" },
				origin: window.location.origin,
			}),
		);

		// Non-string toml
		dispatchConfigMessage(frame, { type: "modconfig-save", toml: 42 });

		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(saveRawConfig).not.toHaveBeenCalled();
	});
});
