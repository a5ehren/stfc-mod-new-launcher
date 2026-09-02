<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = withDefaults(defineProps<{ paused?: boolean }>(), {
	paused: false,
});

const canvas = ref<HTMLCanvasElement | null>(null);
const visible = ref(!props.paused);
const targetFrameTime = 1000 / 15;
let frame = 0;
let lastFrameTime = 0;
let reducedMotion: MediaQueryList | null = null;
let background: CanvasGradient | null = null;
let canvasWidth = 0;
let canvasHeight = 0;
let windowFocused = true;
let stars: Star[] = [];

type Star = {
	x: number;
	y: number;
	z: number;
	pz: number;
	hue: number;
};

function reset(star: Star, width: number, height: number, randomDepth = false) {
	const angle = Math.random() * Math.PI * 2;
	const radius = Math.random() ** 0.55 * Math.max(width, height) * 0.78 + 8;
	star.x = Math.cos(angle) * radius;
	star.y = Math.sin(angle) * radius;
	star.z = randomDepth ? Math.random() * width + 1 : width;
	star.pz = star.z;
	star.hue = 195 + Math.random() * 25;
}

function resize() {
	const element = canvas.value;
	if (!element) return;
	const bounds = element.getBoundingClientRect();
	const ratio = Math.min(window.devicePixelRatio || 1, 1.5);
	canvasWidth = bounds.width;
	canvasHeight = bounds.height;
	element.width = Math.max(1, Math.round(bounds.width * ratio));
	element.height = Math.max(1, Math.round(bounds.height * ratio));
	const context = element.getContext("2d");
	if (!context) return;
	context.setTransform(ratio, 0, 0, ratio, 0, 0);
	background = context.createRadialGradient(
		bounds.width / 2,
		bounds.height / 2,
		0,
		bounds.width / 2,
		bounds.height / 2,
		Math.max(bounds.width, bounds.height) * 0.72,
	);
	background.addColorStop(0, "#092442");
	background.addColorStop(0.18, "#03152b");
	background.addColorStop(1, "#000208");
	stars = Array.from({ length: 120 }, () => {
		const star = {} as Star;
		reset(star, bounds.width, bounds.height, true);
		return star;
	});
	paint(0);
}

function paint(depthStep: number) {
	const element = canvas.value;
	const context = element?.getContext("2d");
	if (!element || !context) return;
	const width = canvasWidth;
	const height = canvasHeight;
	const cx = width / 2;
	const cy = height / 2;
	context.fillStyle = background ?? "#000208";
	context.fillRect(0, 0, width, height);
	for (const star of stars) {
		star.pz = star.z;
		star.z -= depthStep;
		if (star.z < 1) reset(star, width, height);
		const x = cx + (star.x / star.z) * width;
		const y = cy + (star.y / star.z) * width;
		const previousX = cx + (star.x / star.pz) * width;
		const previousY = cy + (star.y / star.pz) * width;
		const speed = 1 - star.z / width;
		context.beginPath();
		context.moveTo(previousX, previousY);
		context.lineTo(x, y);
		context.strokeStyle = `hsla(${star.hue}, 100%, ${72 + speed * 25}%, ${0.18 + speed * 0.82})`;
		context.lineWidth = 0.6 + speed * 3.2;
		context.stroke();
	}
}

function canAnimate() {
	return (
		!props.paused &&
		windowFocused &&
		!document.hidden &&
		!reducedMotion?.matches
	);
}

function requestNextFrame() {
	if (frame === 0 && canAnimate()) frame = requestAnimationFrame(draw);
}

function draw(timestamp: number) {
	frame = 0;
	if (!canAnimate()) return;

	const elapsed =
		lastFrameTime === 0 ? targetFrameTime : timestamp - lastFrameTime;
	if (elapsed < targetFrameTime - 1) {
		requestNextFrame();
		return;
	}

	lastFrameTime = timestamp;
	paint(38 * Math.min(elapsed / (1000 / 60), 4));
	requestNextFrame();
}

function stop() {
	if (frame !== 0) cancelAnimationFrame(frame);
	frame = 0;
	lastFrameTime = 0;
}

function syncAnimation() {
	visible.value = canAnimate();
	if (visible.value) requestNextFrame();
	else stop();
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
	if (navigator.userAgent.includes("jsdom")) return;
	reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
	windowFocused = document.hasFocus();
	resize();
	window.addEventListener("resize", resize);
	window.addEventListener("focus", handleFocus);
	window.addEventListener("blur", handleBlur);
	document.addEventListener("visibilitychange", syncAnimation);
	reducedMotion.addEventListener("change", syncAnimation);
	syncAnimation();
});

watch(() => props.paused, syncAnimation);

onBeforeUnmount(() => {
	window.removeEventListener("resize", resize);
	window.removeEventListener("focus", handleFocus);
	window.removeEventListener("blur", handleBlur);
	document.removeEventListener("visibilitychange", syncAnimation);
	reducedMotion?.removeEventListener("change", syncAnimation);
	stop();
});
</script>

<template>
  <canvas
    ref="canvas"
    class="warp-field"
    :class="{ 'is-hidden': !visible }"
    :aria-hidden="!visible"
    aria-label="Animated warp field"></canvas>
</template>

<style scoped>
.warp-field {
	display: block;
	width: 100%;
	height: 100%;
	opacity: 1;
	transition: opacity 140ms ease;

}
.warp-field.is-hidden { opacity: 0; }

@media (prefers-reduced-motion: reduce) {
	.warp-field { transition: none; }
}
</style>
