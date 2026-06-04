<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import DataCascade from "@/components/lcars/DataCascade.vue";
import LcarsButton from "@/components/lcars/LcarsButton.vue";
import LcarsShell from "@/components/lcars/LcarsShell.vue";
import StatusStrip from "@/components/StatusStrip.vue";
import {
	getLauncherStatus,
	launchGame as launchGameCommand,
	onProgress,
	openLogs,
	openRawConfig,
	setModChannel,
	updateGame as updateGameCommand,
	updateMod as updateModCommand,
} from "@/lib/commands";
import type { LauncherStatus } from "@/types/launcher";

const status = ref<LauncherStatus | null>(null);
const message = ref("Initializing launcher");
let unlistenProgress: (() => void) | null = null;

const channelLabel = computed(() =>
	status.value?.modStatus.channel === "prerelease" ? "Prerelease" : "Stable",
);

const warning = computed(() => {
	if (!status.value) return "";
	if (
		status.value.game.updateAvailable ||
		status.value.modStatus.updateAvailable
	) {
		return "Updates available";
	}
	return "";
});

async function refresh() {
	status.value = await getLauncherStatus();
	message.value = status.value.game.known
		? "Game located"
		: "Game location required on launch";
}

async function launchGame() {
	message.value = warning.value
		? `${warning.value}. Launching anyway.`
		: "Launching game";
	await launchGameCommand();
	message.value = "Game launch started";
}

async function updateGame() {
	message.value = "Checking for game update";
	await updateGameCommand();
	await refresh();
}

async function updateMod() {
	message.value = "Checking for mod update";
	await updateModCommand();
	await refresh();
}

async function toggleChannel() {
	const next =
		status.value?.modStatus.channel === "prerelease" ? "stable" : "prerelease";
	status.value = await setModChannel(next);
}

async function openConfigEditor() {
	const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
	const existing = await WebviewWindow.getByLabel("config-editor");
	if (existing) {
		await existing.setFocus();
		return;
	}
	new WebviewWindow("config-editor", {
		title: "STFC Mod Config",
		url: "/",
		width: 980,
		height: 720,
	});
}

onMounted(async () => {
	unlistenProgress = await onProgress((event) => {
		message.value = event.message;
	});
	await refresh();
});

onBeforeUnmount(() => {
	unlistenProgress?.();
	unlistenProgress = null;
});
</script>

<template>
  <LcarsShell>
    <template #cascade>
      <DataCascade />
    </template>

    <div class="main-grid">
      <div class="actions">
        <LcarsButton tone="orange" @click="launchGame">Launch Game</LcarsButton>
        <LcarsButton v-if="status?.game.updateAvailable" tone="gold" @click="updateGame">Update Game</LcarsButton>
        <LcarsButton v-if="status?.modStatus.updateAvailable" tone="blue" @click="updateMod">Update Mod</LcarsButton>
        <LcarsButton tone="violet" @click="openRawConfig">Open Raw Config</LcarsButton>
        <LcarsButton tone="tan" @click="openConfigEditor">Open Config Editor</LcarsButton>
        <LcarsButton tone="red" @click="openLogs">Open Logs</LcarsButton>
      </div>

      <button class="channel-toggle" @click="toggleChannel">{{ channelLabel }}</button>
      <StatusStrip :message="message" :warning="warning" />
    </div>
  </LcarsShell>
</template>

<style scoped>
.main-grid {
  position: relative;
  height: 100%;
  display: grid;
  grid-template-rows: 1fr auto;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  align-content: start;
  gap: 12px;
  max-width: 460px;
}
.channel-toggle {
  position: absolute;
  right: 18px;
  top: 4px;
  border: 0;
  border-radius: 0 26px 26px 0;
  background: var(--lcars-blue);
  color: #000;
  height: 52px;
  min-width: 130px;
  padding: 0 18px 8px;
  text-transform: uppercase;
  font-weight: 700;
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
}
</style>
