<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = withDefaults(defineProps<{ paused?: boolean }>(), {
	paused: false,
});
const active = ref(!props.paused);
let windowFocused = true;

function syncAnimation() {
	active.value = !props.paused && windowFocused && !document.hidden;
}

function handleFocus() {
	windowFocused = true;
	syncAnimation();
}

function handleBlur() {
	windowFocused = false;
	syncAnimation();
}

onMounted(() => {
	windowFocused = document.hasFocus();
	window.addEventListener("focus", handleFocus);
	window.addEventListener("blur", handleBlur);
	document.addEventListener("visibilitychange", syncAnimation);
	syncAnimation();
});

watch(() => props.paused, syncAnimation);

onBeforeUnmount(() => {
	window.removeEventListener("focus", handleFocus);
	window.removeEventListener("blur", handleBlur);
	document.removeEventListener("visibilitychange", syncAnimation);
});
</script>

<template>
  <svg class="viewscreen-frame" :class="{ 'is-paused': !active }" viewBox="0 0 1511 688" preserveAspectRatio="none" aria-hidden="true">
    <defs>
      <path id="viewscreen-path" pathLength="1320" d="M54 3 H1457 L1508 54 V634 L1457 685 H54 L3 634 V54 Z" />
    </defs>
    <use href="#viewscreen-path" class="rail" />
    <use href="#viewscreen-path" class="pulse" />
    <use href="#viewscreen-path" class="pulse pulse-reverse" />
    <circle class="corner" cx="54" cy="3" r="5" />
    <circle class="corner corner-b" cx="1457" cy="3" r="5" />
    <circle class="corner corner-c" cx="54" cy="685" r="5" />
    <circle class="corner corner-d" cx="1457" cy="685" r="5" />
  </svg>
</template>

<style scoped>
.viewscreen-frame {
	display: block;
	width: 100%;
	height: 100%;
	overflow: visible;
}
.rail,
.pulse {
	fill: none;
	vector-effect: non-scaling-stroke;
}
.rail {
	stroke: #57c9ff;
	stroke-width: 5;
	opacity: 0.34;
}
.pulse {
	stroke: #bcecff;
	stroke-width: 4;
	stroke-linecap: round;
	stroke-dasharray: 70 1250;
	opacity: 0.82;
	animation: frame-travel 7s linear infinite;
}
.pulse-reverse {
	stroke: #188dff;
	stroke-width: 4;
	stroke-dasharray: 28 1292;
	animation-duration: 4.4s;
	animation-direction: reverse;
	opacity: 0.9;
}
.corner {
	fill: #d8f5ff;
	opacity: 0.78;
	animation: corner-breathe 2.8s ease-in-out infinite alternate;
}
.corner-b { animation-delay: -0.7s; }
.corner-c { animation-delay: -1.4s; }
.corner-d { animation-delay: -2.1s; }
.is-paused .pulse,
.is-paused .corner { animation-play-state: paused; }
@keyframes frame-travel {
	to { stroke-dashoffset: -1320; }
}
@keyframes corner-breathe {
	from { opacity: 0.25; }
	to { opacity: 1; }
}
@media (prefers-reduced-motion: reduce) {
	.pulse,
	.corner { animation: none; }
}
</style>
