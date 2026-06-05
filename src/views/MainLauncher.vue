<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
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
  <LcarsShell banner-text="STFC Community Mod" compact-header>
    <div class="main-grid">
      <div class="footer-actions">
        <div class="primary-row">
          <div class="button-cell">
            <LcarsButton tone="violet" edge="left" @click="openRawConfig">Open Raw Config</LcarsButton>
          </div>
          <div class="separator" aria-hidden="true"></div>
          <div class="button-cell">
            <LcarsButton tone="tan" edge="middle" @click="openConfigEditor">Open Config Editor</LcarsButton>
          </div>
          <div class="separator" aria-hidden="true"></div>
          <div class="button-cell">
            <LcarsButton tone="red" edge="middle" @click="openLogs">Open Logs</LcarsButton>
          </div>
          <div class="separator" aria-hidden="true"></div>
          <div class="launch-cell">
            <div v-if="status?.game.updateAvailable || status?.modStatus.updateAvailable" class="update-stack">
              <LcarsButton
                v-if="status?.game.updateAvailable"
                tone="gold"
                :edge="status?.modStatus.updateAvailable ? 'left' : 'single'"
                @click="updateGame"
              >
                Update Game
              </LcarsButton>
              <LcarsButton
                v-if="status?.modStatus.updateAvailable"
                tone="blue"
                :edge="status?.game.updateAvailable ? 'right' : 'single'"
                @click="updateMod"
              >
                Update Mod
              </LcarsButton>
            </div>
            <StatusStrip class="launch-status" :message="message" :warning="warning" />
            <LcarsButton tone="orange" edge="right" @click="launchGame">Launch Game</LcarsButton>
          </div>
        </div>
      </div>

      <button class="channel-toggle" @click="toggleChannel">{{ channelLabel }}</button>
    </div>
  </LcarsShell>
</template>

<style scoped>
.main-grid {
  position: relative;
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
}
.footer-actions {
  display: grid;
  gap: 12px;
  padding: 0 18px 8px 0;
}
.primary-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 4px minmax(0, 1fr) 4px minmax(0, 1fr) 4px minmax(0, 1fr);
  align-items: end;
}
.button-cell {
  min-width: 0;
}
.button-cell :deep(.lcars-button),
.launch-cell :deep(.lcars-button) {
  width: 100%;
  min-width: 0;
}
.separator {
  width: 4px;
  align-self: stretch;
  background: #000;
}
.launch-cell {
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  align-items: flex-end;
  gap: 8px;
}
.update-stack {
  display: flex;
  justify-content: flex-end;
  width: fit-content;
}
.launch-status {
  width: max-content;
  justify-content: flex-end;
}
.channel-toggle {
  position: absolute;
  right: 18px;
  top: 4px;
  border: 0;
  border-radius: 9999px;
  background: var(--lcars-blue);
  color: #000;
  height: 38px;
  min-width: 96px;
  padding: 0 12px 6px;
  text-transform: uppercase;
  font-size: 14px;
  font-weight: 700;
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
}
</style>
