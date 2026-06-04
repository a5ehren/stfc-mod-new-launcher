<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import ConfigSection from "@/components/config/ConfigSection.vue";
import DataCascade from "@/components/lcars/DataCascade.vue";
import LcarsButton from "@/components/lcars/LcarsButton.vue";
import LcarsShell from "@/components/lcars/LcarsShell.vue";
import { readRawConfig, saveRawConfig } from "@/lib/commands";
import { defaultConfig } from "@/lib/config/defaults";
import { generateToml } from "@/lib/config/toml";
import type { TomlConfig } from "@/lib/config/types";

const config = ref<TomlConfig>(structuredClone(defaultConfig));
const savedToml = ref("");
const showToml = ref(false);
const activeSection = ref<keyof TomlConfig>("control");

const sections = [
	{ key: "control" as const, label: "Control Panel" },
	{ key: "graphics" as const, label: "Graphics Settings" },
	{ key: "shortcuts" as const, label: "Keyboard Shortcuts" },
	{ key: "sync" as const, label: "Sync Options" },
	{ key: "ui" as const, label: "Interface" },
	{ key: "buffs" as const, label: "Buffs" },
	{ key: "config" as const, label: "Configuration" },
	{ key: "patches" as const, label: "Patches" },
] as const;

const generatedToml = computed(() =>
	generateToml(config.value, true, true, true, true),
);
const dirty = computed(() => generatedToml.value !== savedToml.value);
const currentSection = computed(
	() =>
		config.value[activeSection.value] as Record<
			string,
			boolean | number | string
		>,
);
const currentDefinitions = computed(() =>
	Object.keys(currentSection.value).map((key) => ({
		group: activeSection.value as string,
		key,
		label: key,
		type: (typeof currentSection.value[key] === "boolean"
			? "checkbox"
			: typeof currentSection.value[key] === "number"
				? "number"
				: "textbox") as "checkbox" | "number" | "textbox",
		description: key,
	})),
);

function updateField(key: string, value: boolean | number | string) {
	config.value = {
		...config.value,
		[activeSection.value]: {
			...(config.value[activeSection.value] as Record<string, unknown>),
			[key]: value,
		},
	};
}

async function save() {
	await saveRawConfig(generatedToml.value);
	savedToml.value = generatedToml.value;
}

onMounted(async () => {
	const raw = await readRawConfig();
	savedToml.value = raw || generateToml(config.value, true, true, true, true);
});
</script>

<template>
  <LcarsShell>
    <template #cascade>
      <DataCascade />
    </template>

    <div class="editor">
      <aside class="tabs">
        <button
          v-for="section in sections"
          :key="section.key"
          :class="{ active: activeSection === section.key }"
          @click="activeSection = section.key">
          {{ section.label }}
        </button>
      </aside>

      <main class="panel">
        <div class="toolbar">
          <span v-if="dirty">Unsaved changes</span>
          <LcarsButton class="save" tone="orange" :disabled="!dirty" @click="save">Save</LcarsButton>
          <LcarsButton tone="blue" @click="showToml = !showToml">
            {{ showToml ? "Hide TOML Preview" : "Show TOML Preview" }}
          </LcarsButton>
        </div>

        <ConfigSection
          :title="sections.find((section) => section.key === activeSection)?.label ?? 'Config'"
          :definitions="currentDefinitions"
          :section="currentSection"
          @update-field="updateField" />

        <textarea v-if="showToml" readonly :value="generatedToml" />
      </main>
    </div>
  </LcarsShell>
</template>

<style scoped>
.editor {
  display: grid;
  grid-template-columns: 190px 1fr;
  gap: 18px;
  height: 100%;
  min-height: 0;
}
.tabs {
  display: grid;
  align-content: start;
  gap: 8px;
}
.tabs button {
  background: var(--lcars-violet);
  color: #000;
  border: 0;
  min-height: 38px;
  padding: 6px 12px;
  text-align: right;
  font-weight: 700;
}
.tabs button.active {
  background: var(--lcars-orange);
}
.panel {
  min-height: 0;
  overflow: auto;
  display: grid;
  gap: 18px;
  align-content: start;
}
.toolbar {
  display: flex;
  gap: 12px;
  align-items: center;
  color: var(--lcars-gold);
}
textarea {
  min-height: 220px;
  background: #080808;
  color: var(--lcars-tan);
  border: 1px solid var(--lcars-violet);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}
</style>
