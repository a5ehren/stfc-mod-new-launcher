<script setup lang="ts">
import { computed } from "vue";
import type { ConfigDefinition } from "@/lib/config/definitions";
import ConfigField from "./ConfigField.vue";

const props = defineProps<{
	definitions: ConfigDefinition[];
	section: Record<string, boolean | number | string>;
}>();

const emit = defineEmits<{
	updateField: [key: string, value: boolean | number | string];
}>();

const groupedDefinitions = computed(() => {
	const groups: Record<string, ConfigDefinition[]> = {};
	for (const definition of props.definitions) {
		const subgroup = definition.subgroup ?? "General";
		if (!groups[subgroup]) {
			groups[subgroup] = [];
		}
		groups[subgroup].push(definition);
	}

	return Object.entries(groups);
});
</script>

<template>
  <section class="config-section">
    <article
      v-for="[subgroup, subgroupDefinitions] in groupedDefinitions"
      :key="subgroup"
      class="subgroup">
      <h3>{{ subgroup }}</h3>
      <ConfigField
        v-for="definition in subgroupDefinitions"
        :key="definition.key"
        :definition="definition"
        :model-value="section[String(definition.key)] ?? ''"
        @update:model-value="emit('updateField', String(definition.key), $event)" />
    </article>
  </section>
</template>

<style scoped>
.config-section {
  display: grid;
  gap: 16px;
}
.subgroup {
  display: grid;
  gap: 12px;
}
.subgroup h3 {
  color: var(--lcars-gold);
  margin: 0;
  text-transform: uppercase;
  font-size: 19px;
}
</style>
