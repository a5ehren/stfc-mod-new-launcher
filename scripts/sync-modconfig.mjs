import { cp, mkdir, rm, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const launcherRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const modconfigDist = path.resolve(launcherRoot, "../config/dist");
const publicRoot = path.join(launcherRoot, "public");

async function ensureDir(dirPath) {
	await mkdir(dirPath, { recursive: true });
}

async function copyDir(src, dest) {
	try {
		await stat(src);
	} catch {
		throw new Error(`Source directory does not exist: ${src}`);
	}
	await cp(src, dest, { recursive: true });
}

async function copyFile(src, dest) {
	try {
		await stat(src);
	} catch {
		throw new Error(`Source file does not exist: ${src}`);
	}
	await cp(src, dest);
}

try {
	await rm(path.join(publicRoot, "_astro"), { recursive: true, force: true });
	await rm(path.join(publicRoot, "flags"), { recursive: true, force: true });
	await ensureDir(path.join(publicRoot, "modconfig"));
	await copyDir(path.join(modconfigDist, "_astro"), path.join(publicRoot, "_astro"));
	await copyDir(path.join(modconfigDist, "flags"), path.join(publicRoot, "flags"));
	await copyFile(
		path.join(modconfigDist, "index.html"),
		path.join(publicRoot, "modconfig/index.html"),
	);
	console.log("Synced the modconfig build into launcher/public.");
} catch (error) {
	console.error("Sync failed:", error instanceof Error ? error.message : error);
	process.exit(1);
}
