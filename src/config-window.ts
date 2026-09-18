import { attachConfigBridge } from "@/lib/configBridge";

const frame = document.getElementById("config-frame") as HTMLIFrameElement;
const banner = document.getElementById("error-banner") as HTMLElement;

attachConfigBridge(frame, (status) => {
	// No status strip in the pop-out; only problems are worth surfacing.
	if (status === "Mod configuration saved") {
		banner.hidden = true;
		return;
	}
	banner.textContent = status;
	banner.hidden = false;
});
