<script setup lang="ts">
import type { ConfigDefinition } from "@/lib/config/definitions";
import ConfigField from "./ConfigField.vue";

const props = defineProps<{
	title: string;
	definitions: ConfigDefinition[];
	section: Record<string, boolean | number | string>;
}>();

const emit = defineEmits<{
	updateField: [key: string, value: boolean | number | string];
}>();
</script>

<template>
  <section class="config-section">
    <h2>{{ title }}</h2>
    <ConfigField
      v-for="definition in definitions"
      :key="definition.key"
      :definition="definition"
      :model-value="section[String(definition.key)] ?? ''"
      @update:model-value="emit('updateField', String(definition.key), $event)" />
  </section>
</template>

<style scoped>
.config-section {
  display: grid;
  gap: 14px;
}
h2 {
  color: var(--lcars-orange);
  margin: 0;
  text-transform: uppercase;
}
</style>
