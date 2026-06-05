<script setup lang="ts">
import { computed } from "vue";
import type { ConfigDefinition } from "@/lib/config/definitions";

const props = defineProps<{
	definition: ConfigDefinition;
	modelValue: boolean | number | string;
}>();

const emit = defineEmits<{
	"update:modelValue": [boolean | number | string];
}>();

const textValue = computed(() =>
	typeof props.modelValue === "string" ? props.modelValue : "",
);
const numberValue = computed(() =>
	typeof props.modelValue === "number" ? props.modelValue : "",
);
const rangeValue = computed(() =>
	typeof props.modelValue === "number" ? props.modelValue : 0,
);
const sliderValue = computed(() =>
	typeof props.modelValue === "number" ? props.modelValue : 0,
);

function update(event: Event) {
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
  <label class="config-field" :title="definition.description ?? definition.label ?? String(definition.key)">
    <span>{{ definition.label }}</span>
    <input
      v-if="definition.type === 'checkbox'"
      type="checkbox"
      :title="definition.description ?? definition.label ?? String(definition.key)"
      :checked="Boolean(modelValue)"
      @change="update" />
    <div v-else-if="definition.type === 'slider'" class="slider-field">
      <input
        type="range"
        :title="definition.description ?? definition.label ?? String(definition.key)"
        :min="definition.min ?? 0"
        :max="definition.max ?? 100"
        :step="definition.step ?? 1"
        :value="rangeValue"
        @input="update" />
      <output>{{ sliderValue }}</output>
    </div>
    <input
      v-else-if="definition.type === 'number'"
      type="number"
      :title="definition.description ?? definition.label ?? String(definition.key)"
      :value="numberValue"
      @input="update" />
    <input
      v-else
      type="text"
      :title="definition.description ?? definition.label ?? String(definition.key)"
      :value="textValue"
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
  font-size: 18px;
}
.slider-field {
  display: grid;
  grid-template-columns: minmax(160px, 1fr) auto;
  gap: 12px;
  align-items: center;
}
output {
  color: var(--lcars-gold);
  font-variant-numeric: tabular-nums;
  justify-self: end;
  min-width: 3ch;
  text-align: right;
}
input {
  background: #111;
  border: 1px solid var(--lcars-violet);
  color: var(--lcars-tan);
  padding: 6px 8px;
  font-size: 16px;
}
input[type="checkbox"] {
  width: 22px;
  height: 22px;
}
</style>
