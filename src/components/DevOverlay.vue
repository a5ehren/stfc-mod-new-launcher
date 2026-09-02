<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useBootstrapStatus } from "@/lib/bootstrap-status";
import { openDevtools } from "@/lib/commands";

const isDev = import.meta.env.DEV;
const { bootstrapStatus, bootstrapError } = useBootstrapStatus();

const hasError = computed(() => bootstrapError.value !== null);
const animationActive = ref(true);
let windowFocused = true;

const isMac = navigator.userAgent.includes("Mac");
const hintKeys = isMac ? ["Cmd", "Opt", "I"] : ["Ctrl", "Shift", "I"];

function onKeydown(event: KeyboardEvent) {
	const modifierMatch = isMac
		? event.metaKey && event.altKey
		: event.ctrlKey && event.shiftKey;
	if (modifierMatch && event.code === "KeyI") {
		event.preventDefault();
		openDevtools();
	}
}

function syncAnimation() {
	animationActive.value = windowFocused && !document.hidden;
}

function onFocus() {
	windowFocused = true;
	syncAnimation();
}

function onBlur() {
	windowFocused = false;
	syncAnimation();
}

onMounted(() => {
	windowFocused = document.hasFocus();
	window.addEventListener("keydown", onKeydown);
	window.addEventListener("focus", onFocus);
	window.addEventListener("blur", onBlur);
	document.addEventListener("visibilitychange", syncAnimation);
	syncAnimation();
});
onUnmounted(() => {
	window.removeEventListener("keydown", onKeydown);
	window.removeEventListener("focus", onFocus);
	window.removeEventListener("blur", onBlur);
	document.removeEventListener("visibilitychange", syncAnimation);
});
</script>

<template>
  <div v-if="isDev" class="dev-overlay" :class="{ 'has-error': hasError, 'is-paused': !animationActive }">
    <div class="dev-overlay__panel">
      <div class="dev-overlay__label">Dev Status</div>
      <div class="dev-overlay__status">{{ bootstrapStatus }}</div>
      <pre v-if="bootstrapError" class="dev-overlay__error">{{ bootstrapError }}</pre>
      <div v-else class="dev-overlay__hint">Open DevTools with <template v-for="(key, index) in hintKeys" :key="key"><span v-if="index > 0"> + </span><kbd>{{ key }}</kbd></template></div>
    </div>
  </div>
</template>

<style scoped>
.dev-overlay {
  position: fixed;
  right: 16px;
  bottom: var(--footer-control-centerline);
  transform: translateY(50%);
  z-index: 9999;
  max-width: min(720px, calc(100vw - 32px));
  pointer-events: none;
}
.dev-overlay__panel {
	position: relative;
	isolation: isolate;
	overflow: hidden;
	pointer-events: auto;
	border: 1px solid rgba(87, 201, 255, 0.48);
	background:
		linear-gradient(135deg, rgba(18, 51, 80, 0.52), transparent 46%),
		rgba(1, 9, 22, 0.92);
	color: #bcecff;
	padding: 16px 20px 14px;
	clip-path: polygon(14px 0, calc(100% - 14px) 0, 100% 14px, 100% calc(100% - 14px), calc(100% - 14px) 100%, 14px 100%, 0 calc(100% - 14px), 0 14px);
	box-shadow: 0 0 18px rgba(87, 201, 255, 0.22), 0 16px 42px rgba(0, 0, 0, 0.65);
}
.dev-overlay__panel::before {
	content: "";
	position: absolute;
	inset: 0;
	z-index: -1;
	background:
		linear-gradient(90deg, transparent 0 15%, #d8f5ff 24%, #188dff 30%, transparent 39%) 0 0 / 240% 2px no-repeat,
		linear-gradient(270deg, transparent 0 15%, #d8f5ff 24%, #188dff 30%, transparent 39%) 100% 100% / 240% 2px no-repeat;
	animation: dev-frame-travel 7s linear infinite;
	pointer-events: none;
}
.is-paused .dev-overlay__panel::before { animation-play-state: paused; }
.dev-overlay__panel::after {
	content: "";
	position: absolute;
	inset: 6px;
	z-index: -1;
	border: 1px solid rgba(87, 201, 255, 0.13);
	clip-path: inherit;
	pointer-events: none;
}
.has-error .dev-overlay__panel {
	border-color: rgba(255, 96, 96, 0.78);
	box-shadow: 0 0 22px rgba(255, 72, 72, 0.28), 0 16px 42px rgba(0, 0, 0, 0.65);
}
.dev-overlay__label {
	color: #79d9ff;
	font-size: 12px;
	text-transform: uppercase;
	letter-spacing: 0.24em;
	margin-bottom: 7px;
	text-shadow: 0 0 10px rgba(87, 201, 255, 0.7);
}
.dev-overlay__status {
	color: #eefbff;
	font-size: 16px;
	line-height: 1.35;
	text-shadow: 0 0 12px rgba(87, 201, 255, 0.38);
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
	color: #80b9d4;
	font-size: 12px;
}
kbd {
	border: 1px solid rgba(87, 201, 255, 0.42);
	border-bottom-width: 2px;
	border-radius: 6px;
	padding: 0 6px;
	font: inherit;
	color: #d8f5ff;
	background: rgba(24, 141, 255, 0.12);
}
@keyframes dev-frame-travel {
	to { background-position: 240% 0, -140% 100%; }
}
@media (prefers-reduced-motion: reduce) {
	.dev-overlay__panel::before { animation: none; }
}
</style>
