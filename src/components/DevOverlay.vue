<script setup lang="ts">
import { computed } from "vue";
import { useBootstrapStatus } from "@/lib/bootstrap-status";

const isDev = import.meta.env.DEV;
const { bootstrapStatus, bootstrapError } = useBootstrapStatus();

const hasError = computed(() => bootstrapError.value !== null);
</script>

<template>
  <div v-if="isDev" class="dev-overlay" :class="{ 'has-error': hasError }">
    <div class="dev-overlay__panel">
      <div class="dev-overlay__label">Dev Status</div>
      <div class="dev-overlay__status">{{ bootstrapStatus }}</div>
      <pre v-if="bootstrapError" class="dev-overlay__error">{{ bootstrapError }}</pre>
      <div v-else class="dev-overlay__hint">Open DevTools with <kbd>Cmd</kbd> + <kbd>Opt</kbd> + <kbd>I</kbd></div>
    </div>
  </div>
</template>

<style scoped>
.dev-overlay {
  position: fixed;
  left: 16px;
  bottom: 16px;
  z-index: 9999;
  max-width: min(720px, calc(100vw - 32px));
  pointer-events: none;
}
.dev-overlay__panel {
  pointer-events: auto;
  border: 1px solid rgba(255, 160, 0, 0.45);
  background: rgba(0, 0, 0, 0.86);
  color: var(--lcars-tan);
  padding: 14px 16px 12px;
  border-radius: 18px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(10px);
}
.has-error .dev-overlay__panel {
  border-color: rgba(255, 96, 96, 0.65);
}
.dev-overlay__label {
  color: var(--lcars-orange);
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  margin-bottom: 6px;
}
.dev-overlay__status {
  color: var(--lcars-text);
  font-size: 16px;
  line-height: 1.35;
}
.dev-overlay__error {
  margin: 10px 0 0;
  white-space: pre-wrap;
  font-size: 12px;
  line-height: 1.45;
  color: #ffd0d0;
  max-width: 64ch;
}
.dev-overlay__hint {
  margin-top: 10px;
  color: var(--lcars-tan);
  font-size: 12px;
}
kbd {
  border: 1px solid rgba(255, 255, 255, 0.18);
  border-bottom-width: 2px;
  border-radius: 6px;
  padding: 0 6px;
  font: inherit;
}
</style>
