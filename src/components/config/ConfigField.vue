<script setup lang="ts">
import type { ConfigDefinition } from "@/lib/config/definitions";

const props = defineProps<{
	definition: ConfigDefinition;
	modelValue: boolean | number | string;
}>();

const emit = defineEmits<{
	"update:modelValue": [boolean | number | string];
}>();

function _update(event: Event) {
	const target = event.target as HTMLInputElement;
	if (props.definition.type === "checkbox") {
		emit("update:modelValue", target.checked);
	} else if (
		props.definition.type === "number" ||
		props.definition.type === "slider"
	) {
		emit("update:modelValue", Number(target.value));
	} else {
		emit("update:modelValue", target.value);
	}
}
</script>

<template>
  <label class="config-field">
    <span>{{ definition.key }}</span>
    <input
      v-if="definition.type === 'checkbox'"
      type="checkbox"
      :checked="Boolean(modelValue)"
      @change="update" />
    <input
      v-else-if="definition.type === 'slider'"
      type="range"
      :min="definition.min ?? 0"
      :max="definition.max ?? 100"
      :step="definition.step ?? 1"
      :value="Number(modelValue)"
      @input="update" />
    <input
      v-else-if="definition.type === 'number'"
      type="number"
      :value="Number(modelValue)"
      @input="update" />
    <input
      v-else
      type="text"
      :value="String(modelValue ?? '')"
      @input="update" />
  </label>
</template>

<style scoped>
.config-field {
  display: grid;
  grid-template-columns: 1fr minmax(160px, 260px);
  gap: 12px;
  align-items: center;
  color: var(--lcars-tan);
}
input {
  background: #111;
  border: 1px solid var(--lcars-violet);
  color: var(--lcars-tan);
  padding: 6px 8px;
}
input[type="checkbox"] {
  width: 22px;
  height: 22px;
}
</style>