<script setup lang="ts">
defineProps<{
	tone?: "violet" | "orange" | "tan" | "blue" | "red" | "gold";
	disabled?: boolean;
}>();

const emit = defineEmits<{ click: [] }>();
</script>

<template>
  <button
    class="lcars-button"
    :class="[`tone-${tone ?? 'violet'}`]"
    :disabled="disabled"
    @click="emit('click')">
    <slot />
  </button>
</template>

<style scoped>
.lcars-button {
  min-width: 132px;
  height: 52px;
  border: 0;
  border-radius: 26px 0 0 26px;
  color: #000;
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
  padding: 0 18px 8px 12px;
  font-weight: 700;
  text-transform: uppercase;
  cursor: pointer;
}
.lcars-button:disabled {
  opacity: 0.45;
  cursor: default;
}
.tone-violet { background: var(--lcars-violet); }
.tone-orange { background: var(--lcars-orange); }
.tone-tan { background: var(--lcars-tan); }
.tone-blue { background: var(--lcars-blue); }
.tone-red { background: var(--lcars-red); }
.tone-gold { background: var(--lcars-gold); }
</style>
