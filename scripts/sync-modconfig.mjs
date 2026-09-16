import { cp, mkdir, readFile, readdir, rm, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const launcherRoot = path.resolve(
	path.dirname(fileURLToPath(import.meta.url)),
	"..",
);
// Override with `pnpm sync:modconfig /path/to/config` or MODCONFIG_DIR.
const modconfigRoot = path.resolve(
	process.argv[2] ?? process.env.MODCONFIG_DIR ?? path.join(launcherRoot, "../config"),
);
const modconfigDist = path.join(modconfigRoot, "dist");
const publicRoot = path.join(launcherRoot, "public");

async function copyDir(src, dest) {
	try {
		await stat(src);
	} catch {
		throw new Error(`Source directory does not exist: ${src}`);
	}
	await cp(src, dest, { recursive: true });
}

try {
	// Validate the source build exists BEFORE wiping any targets; a missing
	// dist must not leave public/_astro|flags|modconfig empty.
	for (const rel of ["_astro", "flags", "index.html"]) {
		try {
			await stat(path.join(modconfigDist, rel));
		} catch {
			throw new Error(
				`Missing ${rel} in ${modconfigDist}; run "pnpm build" in the ModConfig checkout first`,
			);
		}
	}
	// Clean all three targets so stale hashed files from older builds cannot linger.
	for (const dir of ["_astro", "flags", "modconfig"]) {
		const target = path.join(publicRoot, dir);
		await rm(target, { recursive: true, force: true });
		await mkdir(target, { recursive: true });
	}
	await copyDir(path.join(modconfigDist, "_astro"), path.join(publicRoot, "_astro"));
	await copyDir(path.join(modconfigDist, "flags"), path.join(publicRoot, "flags"));
	await cp(
		path.join(modconfigDist, "index.html"),
		path.join(publicRoot, "modconfig/index.html"),
	);

	// Fail loudly if the build predates the launcher bridge.
	const html = await readFile(path.join(publicRoot, "modconfig/index.html"), "utf8");
	if (!/get\(['"]launcher['"]\)/.test(html)) {
		throw new Error(
			"dist/index.html lacks the ?launcher=1 hook; rebuild ModConfig from a revision that includes it",
		);
	}
	const astroFiles = await readdir(path.join(publicRoot, "_astro"));
	const bundles = astroFiles.filter((f) => /\.js$/.test(f));
	const bridge = (
		await Promise.all(
			bundles.map(async (f) => {
				const content = await readFile(path.join(publicRoot, "_astro", f), "utf8");
				return content.includes("modconfig-ready") ? f : null;
			}),
		)
	).find(Boolean);
	if (!bridge) {
		throw new Error(
			"No ModConfig bundle contains the launcher message bridge (modconfig-ready)",
		);
	}

	console.log(`Synced the modconfig build from ${modconfigRoot} into launcher/public.`);
} catch (error) {
	console.error("Sync failed:", error instanceof Error ? error.message : error);
	process.exit(1);
}
