import { cp, mkdir, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const launcherRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const modconfigDist = path.resolve(launcherRoot, "../config/dist");
const publicRoot = path.join(launcherRoot, "public");

await rm(path.join(publicRoot, "_astro"), { recursive: true, force: true });
await rm(path.join(publicRoot, "flags"), { recursive: true, force: true });
await mkdir(path.join(publicRoot, "modconfig"), { recursive: true });
await cp(path.join(modconfigDist, "_astro"), path.join(publicRoot, "_astro"), {
	recursive: true,
});
await cp(path.join(modconfigDist, "flags"), path.join(publicRoot, "flags"), {
	recursive: true,
});
await cp(
	path.join(modconfigDist, "index.html"),
	path.join(publicRoot, "modconfig/index.html"),
);

console.log("Synced the modconfig build into launcher/public.");
