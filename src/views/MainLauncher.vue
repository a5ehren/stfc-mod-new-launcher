<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import ViewscreenButton from "@/components/briefing/ViewscreenButton.vue";
import ViewscreenFrame from "@/components/briefing/ViewscreenFrame.vue";
import WarpField from "@/components/briefing/WarpField.vue";
import InstancePanel from "@/components/InstancePanel.vue";
import StatusStrip from "@/components/StatusStrip.vue";
import {
	checkGameUpdate,
	checkLauncherUpdate,
	checkModUpdate,
	getLauncherStatus,
	installLauncherUpdate,
	launchGame as launchGameCommand,
	onProgress,
	openLogs,
	openRawConfig,
	readRawConfig,
	saveRawConfig,
	setGamePath,
	setModChannel,
	updateGame as updateGameCommand,
	updateMod as updateModCommand,
} from "@/lib/commands";
import { formatError } from "@/lib/formatError";
import type { LauncherStatus } from "@/types/launcher";
import MultiInstanceWizard from "@/views/MultiInstanceWizard.vue";

const status = ref<LauncherStatus | null>(null);
const message = ref("Initializing launcher");
const showWizard = ref(false);
const showConfig = ref(false);
const configFrame = ref<HTMLIFrameElement | null>(null);
let unlistenProgress: (() => void) | null = null;

const warning = computed(() => {
	if (!status.value) return "";
	if (
		status.value.game.updateAvailable ||
		status.value.modStatus.updateAvailable ||
		status.value.launcherUpdateAvailable
	) {
		return "Updates available";
	}
	return "";
});

type UpdateAction = {
	key: string;
	label: string;
	tone: "gold" | "blue" | "red";
	run: () => Promise<void>;
};

const updateActions = computed<UpdateAction[]>(() => {
	const actions: UpdateAction[] = [];
	if (status.value?.game.updateAvailable) {
		actions.push({
			key: "game",
			label: "Update Game",
			tone: "gold",
			run: updateGame,
		});
	}
	if (status.value?.modStatus.updateAvailable) {
		actions.push({
			key: "mod",
			label: "Update Mod",
			tone: "blue",
			run: updateMod,
		});
	}
	if (status.value?.launcherUpdateAvailable) {
		actions.push({
			key: "launcher",
			label: "Update Launcher",
			tone: "red",
			run: updateLauncher,
		});
	}
	return actions;
});

function updateEdge(index: number, total: number) {
	if (total === 1) return "single";
	if (index === 0) return "left";
	if (index === total - 1) return "right";
	return "middle";
}

async function refresh() {
	// Reconcile game/mod/launcher status against remote release sources; each
	// check is best-effort (offline, unknown game path) and falls back to the
	// local snapshot.
	await Promise.allSettled([
		checkGameUpdate(),
		checkModUpdate(),
		checkLauncherUpdate(),
	]);
	status.value = await getLauncherStatus();
	message.value = status.value.game.known
		? "Game located"
		: "Game location required on launch";
}

async function launchGame() {
	message.value = warning.value
		? `${warning.value}. Launching anyway.`
		: "Launching game";
	const result = await runCommandWithGamePathFallback(
		launchGameCommand,
		"Launch cancelled: no game folder selected",
		"Launch failed",
	);
	if (result.ok) {
		message.value = "Game launch started";
	}
}

function isLauncherErrorKind(error: unknown, kind: string): boolean {
	return (
		typeof error === "object" &&
		error !== null &&
		"kind" in error &&
		(error as { kind?: unknown }).kind === kind
	);
}

async function promptForGamePath() {
	const selected = await open({
		directory: true,
		multiple: false,
		title: "Select STFC game folder",
	});

	if (!selected || Array.isArray(selected)) {
		return false;
	}

	try {
		await setGamePath(selected);
		return true;
	} catch (error) {
		if (isLauncherErrorKind(error, "invalidData")) {
			message.value = "Selected folder was not a valid STFC game folder";
			return false;
		}
		throw error;
	}
}

async function updateGame() {
	message.value = "Checking for game update";
	const result = await runCommandWithGamePathFallback(
		updateGameCommand,
		"Update cancelled: no game folder selected",
		"Update failed",
	);
	if (result.ok) {
		await refresh();
		message.value = result.value
			? "Game update complete"
			: "Game already up to date";
	}
}

async function updateMod() {
	message.value = "Checking for mod update";
	await updateModCommand();
	await refresh();
}

async function updateLauncher() {
	message.value = "Downloading launcher update";
	const installed = await installLauncherUpdate();
	if (installed) {
		message.value = "Launcher update installed; restarting";
		await relaunch();
	} else {
		message.value = "Launcher is already up to date";
		await refresh();
	}
}

async function toggleChannel() {
	const next =
		status.value?.modStatus.channel === "prerelease" ? "stable" : "prerelease";
	status.value = await setModChannel(next);
	try {
		status.value = await checkModUpdate();
	} catch {
		// channel label still updated; availability unknown until next check
	}
}

async function runCommandWithGamePathFallback<T>(
	command: () => Promise<T>,
	cancelMessage: string,
	failureLabel: string,
): Promise<{ ok: boolean; value?: T }> {
	try {
		const value = await command();
		return { ok: true, value };
	} catch (error) {
		if (isLauncherErrorKind(error, "gamePath")) {
			try {
				const selected = await promptForGamePath();
				if (selected) {
					const value = await command();
					return { ok: true, value };
				}
				message.value = cancelMessage;
			} catch (promptError) {
				message.value = `${failureLabel}: ${formatError(promptError)}`;
			}
			return { ok: false };
		}
		message.value = `${failureLabel}: ${formatError(error)}`;
		return { ok: false };
	}
}

async function onWizardDone() {
	showWizard.value = false;
	await refresh();
}

async function handleConfigMessage(event: MessageEvent) {
	if (event.source !== configFrame.value?.contentWindow) return;
	if (event.origin !== window.location.origin) return;

	if (event.data?.type === "modconfig-ready") {
		const toml = await readRawConfig();
		configFrame.value?.contentWindow?.postMessage(
			{ type: "stfc-launcher-config", toml },
			window.location.origin,
		);
	}

	if (
		event.data?.type === "modconfig-save" &&
		typeof event.data.toml === "string"
	) {
		await saveRawConfig(event.data.toml);
		message.value = "Mod configuration saved";
	}
}

onMounted(async () => {
	window.addEventListener("message", handleConfigMessage);
	unlistenProgress = await onProgress((event) => {
		message.value = event.message;
	});
	await refresh();
});

onBeforeUnmount(() => {
	window.removeEventListener("message", handleConfigMessage);
	unlistenProgress?.();
	unlistenProgress = null;
});
</script>

<template>
  <MultiInstanceWizard v-if="showWizard" @done="onWizardDone" />
  <section v-else class="lcars-shell compact-header briefing-room">
	<Transition name="config-lower">
	  <aside v-if="showConfig" class="config-drawer" aria-label="Mod configuration editor">
		<header class="config-drawer__bar">
		  <div>
			<strong>MODCONFIG</strong>
			<span>Launcher edition</span>
		  </div>
		  <button type="button" aria-label="Close config editor" @click="showConfig = false">Close</button>
		</header>
		<iframe
		  ref="configFrame"
		  title="STFC Mod Config"
		  src="/modconfig/index.html?launcher=1"
		/>
		<div class="config-drawer__rail" aria-hidden="true"><span></span></div>
	  </aside>
	</Transition>
    <div class="viewscreen">
      <WarpField :paused="showConfig" />
      <div class="screen-interface">
        <div class="title-block">
          <span class="kicker">STFC Community Mod // 1701</span>
          <h1>Launcher <span class="kicker">v<span class="kicker-larger">0.0.1</span></span></h1>
        </div>
        <div class="screen-status">
          <span class="status-light" :class="{ warning: warning }"></span>
          <StatusStrip class="launch-status" :message="message" :warning="warning" />
        </div>
        <div class="screen-actions">
          <button
            class="channel-toggle"
            :aria-pressed="status?.modStatus.channel === 'prerelease'"
            title="Switch mod channel"
            @click="toggleChannel"
          >
            <span :class="{ active: status?.modStatus.channel !== 'prerelease' }">Stable</span>
            <span :class="{ active: status?.modStatus.channel === 'prerelease' }">Prerelease</span>
          </button>
          <div v-if="updateActions.length > 0" class="update-stack">
            <ViewscreenButton
              v-for="(action, index) in updateActions"
              :key="action.key"
              :tone="action.tone"
              :edge="updateEdge(index, updateActions.length)"
              @click="action.run"
            >{{ action.label }}</ViewscreenButton>
          </div>
        </div>
        <InstancePanel v-if="status?.multiInstance?.enabled" />
      </div>
      <ViewscreenFrame class="viewscreen-effects" :paused="showConfig" />
    </div>

	<div class="room-actions" aria-label="Launcher controls">
		<ViewscreenButton variant="console" tone="tan" edge="single" @click="showConfig = true">Open Config Editor</ViewscreenButton>
		<ViewscreenButton variant="console" tone="violet" edge="single" @click="openRawConfig">Open Raw Config</ViewscreenButton>
      <ViewscreenButton variant="console" tone="red" edge="single" @click="openLogs">Open Logs</ViewscreenButton>
      <ViewscreenButton variant="console" tone="blue" edge="single" @click="showWizard = true">Multi-Instance</ViewscreenButton>
      <ViewscreenButton variant="console" tone="orange" edge="single" @click="launchGame">Launch Game</ViewscreenButton>
    </div>
  </section>
</template>

<style scoped>
.briefing-room {
	position: relative;
	width: 100vw;
	height: 100vh;
	box-sizing: border-box;
	overflow: hidden;
	background: #020914 url("@/assets/briefing-room/backplate-chairless.png") center / 100% 100% no-repeat;
}
.config-drawer {
	position: absolute;
	z-index: 20;
	top: 0;
	left: 5%;
	right: 5%;
	height: min(82vh, 660px);
	display: grid;
	grid-template-rows: 44px minmax(0, 1fr) 18px;
	background: rgba(9, 10, 12, 0.98);
	border: 1px solid rgba(255, 255, 255, 0.18);
	border-top: 0;
	border-radius: 0 0 18px 18px;
	box-shadow: 0 24px 70px rgba(0, 0, 0, 0.78), 0 0 26px rgba(235, 148, 58, 0.2);
	overflow: hidden;
}
.config-drawer__bar {
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 0 14px 0 18px;
	background: linear-gradient(90deg, #15171a, #202126 70%, #111214);
	border-bottom: 1px solid rgba(255, 255, 255, 0.12);
	color: #f7f7f7;
	letter-spacing: 0.08em;
}
.config-drawer__bar div { display: flex; align-items: baseline; gap: 10px; }
.config-drawer__bar strong { color: var(--lcars-orange); font-size: 17px; }
.config-drawer__bar span { color: #8d939d; font-size: 11px; text-transform: uppercase; }
.config-drawer__bar button {
	border: 1px solid rgba(255, 255, 255, 0.22);
	border-radius: 7px;
	background: #24262b;
	color: #f2f2f2;
	padding: 5px 13px;
	text-transform: uppercase;
	font-size: 11px;
	font-weight: 700;
	cursor: pointer;
}
.config-drawer__bar button:hover,
.config-drawer__bar button:focus-visible { border-color: var(--lcars-orange); outline: none; }
.config-drawer iframe { width: 100%; height: 100%; border: 0; background: #101113; }
.config-drawer__rail {
	display: flex;
	align-items: center;
	justify-content: center;
	background: linear-gradient(180deg, #25272b, #111214);
	border-top: 1px solid rgba(255, 255, 255, 0.14);
}
.config-drawer__rail span { width: 90px; height: 3px; border-radius: 3px; background: #73777e; box-shadow: 0 0 8px rgba(255, 255, 255, 0.16); }
.config-lower-enter-active { animation: lower-panel 620ms cubic-bezier(0.2, 0.82, 0.24, 1); }
.config-lower-leave-active { animation: lower-panel 360ms cubic-bezier(0.62, 0, 0.78, 0.24) reverse; }
@keyframes lower-panel {
	0% { transform: translateY(calc(-100% - 32px)); }
	72% { transform: translateY(8px); }
	100% { transform: translateY(0); }
}
.viewscreen {
	position: absolute;
	left: 10.3%;
	top: 7.2%;
	width: 79.2%;
	height: 64.7%;
	overflow: hidden;
	clip-path: polygon(3.6% 0, 96.4% 0, 100% 7.5%, 100% 92.5%, 96.4% 100%, 3.6% 100%, 0 92.5%, 0 7.5%);
}
.screen-interface {
	position: absolute;
	inset: 0;
	display: grid;
	grid-template-columns: 1fr auto;
	grid-template-rows: auto 1fr auto;
	padding: clamp(18px, 3vw, 48px);
	box-sizing: border-box;
	background: linear-gradient(90deg, rgba(0, 6, 18, 0.78), transparent 58%);
}
.title-block {
	align-self: start;
}
.kicker {
	color: #79d9ff;
	font-size: clamp(10px, 1vw, 15px);
	letter-spacing: 0.24em;
	 text-transform: uppercase;
	font-variant-caps: small-caps;
}

.kicker-larger {
	font-size: clamp(20px, 2vw, 30px);
}

h1 {
	margin: 8px 0 0;
	color: #eefbff;
	font-size: clamp(26px, 4vw, 66px);
	line-height: 0.9;
	text-transform: uppercase;
	text-shadow: 0 0 22px rgba(75, 202, 255, 0.65);
}
.screen-status {
	grid-column: 1 / -1;
	align-self: end;
	display: flex;
	align-items: center;
	gap: 10px;
	padding-bottom: 12px;
}
.status-light {
	width: 8px;
	height: 8px;
	border-radius: 50%;
	background: #62e6b2;
	box-shadow: 0 0 12px #62e6b2;
}
.status-light.warning { background: var(--lcars-gold); box-shadow: 0 0 12px var(--lcars-gold); }
.launch-status { color: #d9f5ff; font-size: clamp(13px, 1.3vw, 18px); }
.screen-actions {
	grid-column: 2;
	grid-row: 1;
	display: flex;
	flex-direction: column;
	align-items: flex-end;
	gap: 8px;
	align-self: start;
}
.update-stack { display: flex; }
.update-stack :deep(.viewscreen-button) {
	height: 38px;
	min-width: 110px;
	font-size: 12px;
}
.viewscreen-effects {
	position: absolute;
	inset: 0;
	z-index: 3;
	pointer-events: none;
}
.room-actions {
	position: absolute;
	left: 5.5%;
	right: 25%;
	bottom: var(--footer-control-centerline);
	transform: translateY(50%);
	display: grid;
	grid-template-columns: repeat(5, 1fr);
	gap: clamp(10px, 2vw, 32px);
}
.room-actions :deep(.viewscreen-button) {
	width: 100%;
	max-height: 80%;
	min-width: 0;
}
.screen-interface :deep(.instance-panel) {
	grid-column: 1 / -1;
	grid-row: 2;
	align-self: center;
	max-height: 44%;
	overflow: auto;
	background: rgba(1, 10, 24, 0.78);
	border: 1px solid rgba(101, 211, 255, 0.35);
	border-radius: 10px;
	padding: 12px;
}
.channel-toggle {
	position: relative;
	border: 1px solid #63cfff;
	border-radius: 999px;
	background: rgba(1, 10, 24, 0.86);
	padding: 3px;
	display: flex;
	gap: 3px;
	cursor: pointer;
	text-transform: uppercase;
	font-size: 11px;
	font-weight: 700;
	overflow: hidden;
	box-shadow: 0 0 12px rgba(87, 201, 255, 0.2);
	transition: transform 120ms ease, box-shadow 160ms ease, filter 120ms ease;
}
.channel-toggle::before {
	content: "";
	position: absolute;
	inset: -1px;
	border-radius: inherit;
	background: linear-gradient(90deg, transparent, #d8f5ff, #188dff, transparent) 0 0 / 45% 2px no-repeat;
	opacity: 0;
	pointer-events: none;
}
.channel-toggle:hover::before,
.channel-toggle:focus-visible::before {
	opacity: 1;
	animation: channel-tracer 1.8s linear infinite;
}
.channel-toggle:hover,
.channel-toggle:focus-visible {
	box-shadow: 0 0 18px rgba(87, 201, 255, 0.55);
	outline: none;
}
.channel-toggle:active {
	transform: translateY(2px) scale(0.97);
	filter: brightness(1.4);
}
.channel-toggle span {
	padding: 5px 9px;
	border-radius: 999px;
	color: #78d8ff;
	opacity: 0.5;
}
.channel-toggle span.active {
	background: #78d8ff;
	color: #03101a;
	opacity: 1;
}
@keyframes channel-tracer {
	to { background-position: 220% 0; }
}
@media (prefers-reduced-motion: reduce) {
	.channel-toggle::before { animation: none !important; }
	.config-lower-enter-active,
	.config-lower-leave-active { animation-duration: 1ms; }
}
@media (max-aspect-ratio: 4 / 3) {
	.briefing-room { background-size: cover; }
	.room-actions { left: 4%; right: 4%; gap: 8px; }
}
</style>
