<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import ConfigSection from "@/components/config/ConfigSection.vue";
import LcarsButton from "@/components/lcars/LcarsButton.vue";
import LcarsShell from "@/components/lcars/LcarsShell.vue";
import { readRawConfig, saveRawConfig } from "@/lib/commands";
import { defaultConfig } from "@/lib/config/defaults";
import { allDefinitions } from "@/lib/config/definitions";
import { generateToml } from "@/lib/config/toml";
import type { TomlConfig } from "@/lib/config/types";

const config = ref<TomlConfig>(structuredClone(defaultConfig));
const savedToml = ref("");
const showToml = ref(false);
const activeSection = ref<keyof TomlConfig>("control");

const sections = [
	{
		key: "control" as const,
		label: "Control Panel",
		description:
			"This section provides options that affect the overall controller of the mod",
	},
	{
		key: "graphics" as const,
		label: "Graphics Settings",
		description: "The graphics settings affect how things are displayed to you",
	},
	{
		key: "shortcuts" as const,
		label: "Keyboard Shortcuts",
		description: "Make each shortcut your own using examples on our github",
	},
	{
		key: "sync" as const,
		label: "Sync Options",
		description:
			"These options are only relevant if you wish to synchronise data automatically with a remote server such as Spocks Club",
	},
	{
		key: "ui" as const,
		label: "Interface",
		description: "Toggle various UI states with these options",
	},
	{
		key: "buffs" as const,
		label: "Buffs",
		description: "Buffs, buffs? Seriously? Nah, these don't work",
	},
	{
		key: "config" as const,
		label: "Configuration",
		description:
			"⚠️⚠️⚠️ DEVELOPER: These settings are used by developers, you shouldn't be touching them!",
	},
	{
		key: "patches" as const,
		label: "Patches",
		description:
			"⚠️⚠️⚠️ SUPPORT: Yeah, don't mess with these unless asked to by the Community Mod support. These should all be enabled by default",
	},
] as const;

const activeSectionDetails = computed(
	() =>
		sections.find((section) => section.key === activeSection.value) ??
		sections[0],
);

const sectionTones: Record<
	keyof TomlConfig,
	"violet" | "orange" | "tan" | "blue" | "red" | "gold"
> = {
	control: "violet",
	graphics: "tan",
	shortcuts: "red",
	sync: "blue",
	ui: "red",
	buffs: "blue",
	config: "violet",
	patches: "red",
};

function humanizeKey(key: string) {
	return key
		.replace(/_/g, " ")
		.replace(/\b\w/g, (character) => character.toUpperCase());
}

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
const currentDefinitions = computed(() => {
	const definitions = allDefinitions.filter(
		(definition) => definition.group?.toLowerCase() === activeSection.value,
	);
	const knownKeys = new Set(
		definitions.map((definition) => String(definition.key)),
	);
	const fallbackDefinitions = Object.keys(currentSection.value)
		.filter((key) => !knownKeys.has(key))
		.map((key) => ({
			group: activeSection.value as string,
			key,
			label: humanizeKey(key),
			type: (typeof currentSection.value[key] === "boolean"
				? "checkbox"
				: typeof currentSection.value[key] === "number"
					? "number"
					: "textbox") as "checkbox" | "number" | "textbox",
			description: `${humanizeKey(key)} setting`,
		}));

	return [...definitions, ...fallbackDefinitions];
});

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
  <LcarsShell banner-text="STFC Mod : Config" compact-header>
    <div class="editor">
      <div class="tabs" aria-label="Config sections">
        <div class="tab-row" role="tablist" aria-label="Config sections row 1">
          <template v-for="(section, index) in sections.slice(0, 4)" :key="section.key">
            <div v-if="index > 0" class="separator" aria-hidden="true"></div>
            <LcarsButton
              class="tab-button"
              :tone="activeSection === section.key ? 'orange' : sectionTones[section.key]"
              :edge="index === 0 ? 'left' : index === 3 ? 'right' : 'middle'"
              :aria-selected="activeSection === section.key"
              :title="section.description"
              role="tab"
              @click="activeSection = section.key">
              {{ section.label }}
            </LcarsButton>
          </template>
        </div>
        <div class="tab-row" role="tablist" aria-label="Config sections row 2">
          <template v-for="(section, index) in sections.slice(4)" :key="section.key">
            <div v-if="index > 0" class="separator" aria-hidden="true"></div>
            <LcarsButton
              class="tab-button"
              :tone="activeSection === section.key ? 'orange' : sectionTones[section.key]"
              :edge="index === 0 ? 'left' : index === 3 ? 'right' : 'middle'"
              :aria-selected="activeSection === section.key"
              :title="section.description"
              role="tab"
              @click="activeSection = section.key">
              {{ section.label }}
            </LcarsButton>
          </template>
        </div>
      </div>

      <main class="panel">
        <div class="panel-header">
          <h2 :title="activeSectionDetails.description">{{ activeSectionDetails.label }}</h2>
          <p>{{ activeSectionDetails.description }}</p>
        </div>

        <div class="toolbar">
          <span v-if="dirty">Unsaved changes</span>
          <LcarsButton class="save compact-action" tone="orange" edge="single" :disabled="!dirty" @click="save">Save</LcarsButton>
          <LcarsButton class="compact-action" tone="blue" edge="single" @click="showToml = !showToml">
            {{ showToml ? "Hide TOML Preview" : "Show TOML Preview" }}
          </LcarsButton>
        </div>

        <div class="panel-scroll">
          <ConfigSection
            :definitions="currentDefinitions"
            :section="currentSection"
            @update-field="updateField" />

          <textarea v-if="showToml" readonly :value="generatedToml" />
        </div>
      </main>
    </div>
  </LcarsShell>
</template>

<style scoped>
.editor {
  display: flex;
  flex-direction: column;
  gap: 18px;
  height: 100%;
  min-height: 0;
}
.tabs {
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow: hidden;
  flex-shrink: 0;
}
.tab-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 4px minmax(0, 1fr) 4px minmax(0, 1fr) 4px minmax(0, 1fr);
  align-items: stretch;
}
.tab-button {
  width: 100%;
  min-width: 0;
  font-size: 14px;
  height: 40px;
  padding-bottom: 6px;
}
.tab-row :deep(.lcars-button) {
  width: 100%;
  min-width: 0;
}
.separator {
  width: 4px;
  background: #000;
}
.panel {
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
}
.panel-header {
  display: grid;
  gap: 4px;
}
.panel-header h2 {
  margin: 0;
  color: var(--lcars-orange);
  text-transform: uppercase;
  font-size: 30px;
  line-height: 1;
}
.panel-header p {
  margin: 0;
  color: var(--lcars-tan);
  font-size: 17px;
  line-height: 1.2;
}
.panel-scroll {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: grid;
  gap: 18px;
  align-content: start;
  padding-right: 2px;
}
.toolbar {
  display: flex;
  gap: 12px;
  align-items: center;
  color: var(--lcars-gold);
  flex-wrap: wrap;
}
.compact-action {
  min-width: 92px;
  height: 34px;
  font-size: 13px;
  padding-bottom: 4px;
}
textarea {
  min-height: 220px;
  background: #080808;
  color: var(--lcars-tan);
  border: 1px solid var(--lcars-violet);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}
</style>
