<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

const canvas = ref<HTMLCanvasElement | null>(null);
let frame = 0;
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
	const radius =
		Math.random() ** 0.55 * Math.max(width, height) * 0.78 + 8;
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
	const ratio = Math.min(window.devicePixelRatio || 1, 2);
	element.width = bounds.width * ratio;
	element.height = bounds.height * ratio;
	const context = element.getContext("2d");
	context?.setTransform(ratio, 0, 0, ratio, 0, 0);
	stars = Array.from({ length: 420 }, () => {
		const star = {} as Star;
		reset(star, bounds.width, bounds.height, true);
		return star;
	});
}

function draw() {
	const element = canvas.value;
	const context = element?.getContext("2d");
	if (!element || !context) return;
	const { width, height } = element.getBoundingClientRect();
	const cx = width / 2;
	const cy = height / 2;
	const background = context.createRadialGradient(
		cx,
		cy,
		0,
		cx,
		cy,
		Math.max(width, height) * 0.72,
	);
	background.addColorStop(0, "#092442");
	background.addColorStop(0.18, "#03152b");
	background.addColorStop(1, "#000208");
	context.fillStyle = background;
	context.fillRect(0, 0, width, height);
	context.globalCompositeOperation = "lighter";
	for (const star of stars) {
		star.pz = star.z;
		star.z -= 38;
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
	context.globalCompositeOperation = "source-over";
	frame = requestAnimationFrame(draw);
}

onMounted(() => {
	if (navigator.userAgent.includes("jsdom")) return;
	resize();
	window.addEventListener("resize", resize);
	frame = requestAnimationFrame(draw);
});

onBeforeUnmount(() => {
	window.removeEventListener("resize", resize);
	cancelAnimationFrame(frame);
});
</script>

<template>
  <canvas ref="canvas" class="warp-field" aria-label="Animated warp field"></canvas>
</template>

<style scoped>
.warp-field {
	display: block;
	width: 100%;
	height: 100%;
}
</style>
