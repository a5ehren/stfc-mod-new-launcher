<script setup lang="ts">
import { confirm } from "@tauri-apps/plugin-dialog";
import { onBeforeUnmount, onMounted, ref } from "vue";
import LcarsButton from "@/components/lcars/LcarsButton.vue";
import {
	miBackupInstance,
	miInstanceStatus,
	miRemoveInstance,
	miRestoreInstance,
	miSetInstanceLabel,
	miStartInstance,
	miStopInstance,
} from "@/lib/commands";
import { formatError } from "@/lib/formatError";
import type { InstanceStatusDto } from "@/types/launcher";

const instances = ref<InstanceStatusDto[]>([]);
const message = ref("");
const editing = ref<string | null>(null);
const editValue = ref("");
let poll: ReturnType<typeof setInterval> | null = null;

function displayName(instance: InstanceStatusDto): string {
	const shown =
		instance.label ?? (instance.isBase ? "Primary (you)" : instance.name);
	return instance.label ? `${shown} (${instance.name})` : shown;
}

function startEdit(instance: InstanceStatusDto) {
	editing.value = instance.name;
	editValue.value = instance.label ?? "";
}

async function saveEdit(name: string) {
	if (editing.value !== name) return;
	editing.value = null;
	await run(() => miSetInstanceLabel(name, editValue.value), "Label updated");
}

async function refresh() {
	try {
		instances.value = await miInstanceStatus();
	} catch (error) {
		message.value = formatError(error);
	}
}

async function run(action: () => Promise<unknown>, done: string) {
	message.value = "";
	try {
		await action();
		message.value = done;
	} catch (error) {
		message.value = formatError(error);
	}
	await refresh();
}

function start(name: string) {
	return run(() => miStartInstance(name), `Started ${name}`);
}

function stop(name: string) {
	return run(() => miStopInstance(name), `Stopped ${name}`);
}

function backup(name: string) {
	return run(() => miBackupInstance(name), `Backed up ${name}`);
}

function restore(name: string) {
	return run(() => miRestoreInstance(name), `Restored ${name}`);
}

async function remove(instance: InstanceStatusDto) {
	if (instance.lastBackupAt === null) {
		const confirmed = await confirm(
			`No backup exists for ${instance.name} — its game account will be unrecoverable. Remove anyway?`,
			{ title: "Remove instance", kind: "warning" },
		);
		if (!confirmed) return;
		await run(
			() => miRemoveInstance(instance.name, true),
			`Removed ${instance.name}`,
		);
		return;
	}
	await run(
		() => miRemoveInstance(instance.name, false),
		`Removed ${instance.name}`,
	);
}

onMounted(async () => {
	await refresh();
	poll = setInterval(refresh, 5000);
});

onBeforeUnmount(() => {
	if (poll) clearInterval(poll);
});
</script>

<template>
  <section class="instance-panel">
    <h2>Instances</h2>
    <p v-if="message" class="message">{{ message }}</p>
    <p v-if="instances.length === 0" class="empty">No instances provisioned.</p>
    <div v-for="instance in instances" :key="instance.name" class="instance-row">
      <span class="name" :title="instance.osUsername" @click="startEdit(instance)">
        <input
          v-if="editing === instance.name"
          v-model="editValue"
          class="label-input"
          maxlength="32"
          placeholder="(name)"
          @keydown.enter="saveEdit(instance.name)"
          @keydown.esc="editing = null"
          @blur="saveEdit(instance.name)"
        />
        <template v-else>{{ displayName(instance) }}</template>
      </span>
      <span class="badge" :class="instance.running ? 'running' : 'stopped'">
        {{ instance.running ? "Running" : "Stopped" }}
      </span>
      <span class="pid">{{ instance.pid ?? "—" }}</span>
      <span class="backup">{{ instance.lastBackupAt ?? "no backup" }}</span>
      <span class="actions">
        <LcarsButton tone="blue" edge="single" :disabled="instance.running" @click="start(instance.name)">Start</LcarsButton>
        <LcarsButton tone="orange" edge="single" :disabled="!instance.running" @click="stop(instance.name)">Stop</LcarsButton>
        <LcarsButton tone="tan" edge="single" @click="backup(instance.name)">Backup</LcarsButton>
        <LcarsButton tone="gold" edge="single" @click="restore(instance.name)">Restore</LcarsButton>
        <LcarsButton v-if="!instance.isBase" tone="red" edge="single" @click="remove(instance)">Remove</LcarsButton>
      </span>
    </div>
  </section>
</template>

<style scoped>
.instance-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 0 18px;
  color: #fff;
}
.instance-panel h2 {
  margin: 0;
  color: var(--lcars-orange);
  text-transform: uppercase;
  font-size: 16px;
}
.message {
  color: var(--lcars-gold);
  margin: 0;
}
.empty {
  color: var(--lcars-blue);
  margin: 0;
}
.instance-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.name {
  min-width: 96px;
  font-weight: 700;
  cursor: text;
}
.label-input {
  background: #000;
  border: 1px solid var(--lcars-blue);
  border-radius: 6px;
  color: #fff;
  padding: 2px 6px;
  font: inherit;
  width: 140px;
}
.badge {
  min-width: 72px;
  text-transform: uppercase;
  font-size: 12px;
}
.badge.running {
  color: var(--lcars-blue);
}
.badge.stopped {
  color: var(--lcars-tan, #cc9966);
}
.pid,
.backup {
  min-width: 96px;
  font-family: monospace;
  font-size: 12px;
  color: #aaa;
}
.actions {
  display: flex;
  gap: 4px;
}
.actions :deep(.lcars-button) {
  min-width: 0;
  height: 32px;
  font-size: 12px;
  padding: 0 10px;
  border-radius: 16px;
}
</style>
