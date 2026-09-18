import { readRawConfig, saveRawConfig } from "@/lib/commands";
import { formatError } from "@/lib/formatError";

// Bridges the modconfig iframe's postMessage protocol to the launcher's Tauri
// config commands. Returns a detach function.
export function attachConfigBridge(
	frame: HTMLIFrameElement,
	onStatus: (message: string) => void,
): () => void {
	const handler = async (event: MessageEvent) => {
		if (event.source !== frame.contentWindow) return;
		if (event.origin !== window.location.origin) return;
		try {
			if (event.data?.type === "modconfig-ready") {
				const toml = await readRawConfig();
				frame.contentWindow?.postMessage(
					{ type: "stfc-launcher-config", toml },
					window.location.origin,
				);
			}
			if (
				event.data?.type === "modconfig-save" &&
				typeof event.data.toml === "string"
			) {
				await saveRawConfig(event.data.toml);
				onStatus("Mod configuration saved");
			} else if (event.data?.type === "modconfig-save") {
				onStatus("Invalid configuration: expected a TOML string");
			}
		} catch (error) {
			onStatus(`Mod configuration failed: ${formatError(error)}`);
		}
	};
	window.addEventListener("message", handler);
	return () => window.removeEventListener("message", handler);
}
