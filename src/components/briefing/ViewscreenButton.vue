<script setup lang="ts">
withDefaults(
	defineProps<{
		tone?: "violet" | "orange" | "tan" | "blue" | "red" | "gold";
		disabled?: boolean;
		edge?: "left" | "middle" | "right" | "single";
		variant?: "compact" | "console";
	}>(),
	{ tone: "violet", edge: "left", variant: "compact" },
);

const emit = defineEmits<{ click: [] }>();
</script>

<template>
  <button
    class="viewscreen-button"
    :class="[`edge-${edge}`, `variant-${variant}`]"
    :style="{ '--control-accent': `var(--lcars-${tone})` }"
    :disabled="disabled"
    @click="emit('click')"
  >
    <span class="viewscreen-button__label"><slot /></span>
  </button>
</template>

<style scoped>
.viewscreen-button {
	position: relative;
	isolation: isolate;
	overflow: hidden;
	min-width: 132px;
	height: 52px;
	border: 1px solid color-mix(in srgb, var(--control-accent) 58%, #d8f5ff);
	background: linear-gradient(180deg, rgba(15, 42, 66, 0.94), rgba(2, 12, 27, 0.97));
	box-shadow:
		inset 0 1px 0 rgba(216, 245, 255, 0.24),
		inset 0 -8px 16px rgba(0, 0, 0, 0.28),
		0 0 12px color-mix(in srgb, var(--control-accent) 22%, transparent);
	color: color-mix(in srgb, var(--control-accent) 62%, white);
	display: flex;
	align-items: center;
	justify-content: center;
	padding: 0 12px;
	font-weight: 800;
	text-transform: uppercase;
	cursor: pointer;
	text-shadow: 0 0 10px color-mix(in srgb, var(--control-accent) 55%, transparent);
	transition: transform 140ms ease, filter 140ms ease, box-shadow 180ms ease;
}
.viewscreen-button__label { position: relative; z-index: 1; }
.edge-left { border-radius: 26px 0 0 26px; }
.edge-middle { border-radius: 0; }
.edge-right { border-radius: 0 26px 26px 0; }
.edge-single { border-radius: 26px; }
.viewscreen-button:disabled { opacity: 0.45; cursor: default; }
.viewscreen-button::before {
	content: "";
	position: absolute;
	inset: -2px;
	z-index: 0;
	border-radius: inherit;
	padding: 2px;
	background: conic-gradient(from 0deg, transparent 0deg 250deg, color-mix(in srgb, var(--control-accent) 70%, white) 276deg, #e8fbff 292deg, transparent 322deg);
	-webkit-mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
	-webkit-mask-composite: xor;
	mask-composite: exclude;
	opacity: 0;
	pointer-events: none;
}
.viewscreen-button::after {
	content: "";
	position: absolute;
	inset: -35%;
	z-index: 0;
	background: radial-gradient(circle, color-mix(in srgb, var(--control-accent) 65%, white) 0 6%, transparent 7% 100%);
	transform: scale(0);
	opacity: 0;
	pointer-events: none;
}
.viewscreen-button:hover::before,
.viewscreen-button:focus-visible::before {
	opacity: 1;
	animation: control-border-travel 2.2s linear infinite;
}
.viewscreen-button:hover,
.viewscreen-button:focus-visible {
	filter: brightness(1.16);
	box-shadow: inset 0 1px 0 rgba(232, 251, 255, 0.38), 0 0 10px color-mix(in srgb, var(--control-accent) 58%, transparent), 0 0 26px color-mix(in srgb, var(--control-accent) 26%, transparent);
	outline: none;
}
.viewscreen-button:active {
	filter: brightness(1.35);
	transform: translateY(3px) scale(0.975);
	box-shadow: inset 0 0 22px color-mix(in srgb, var(--control-accent) 46%, transparent), 0 0 30px color-mix(in srgb, var(--control-accent) 52%, transparent);
	transition-duration: 55ms;
}
.viewscreen-button:active::after { animation: control-activate 420ms ease-out; }
.variant-console {
	height: clamp(64px, 8vw, 112px);
	border-radius: 12px 12px 28px 28px;
	box-shadow: 0 8px 0 rgba(1, 8, 20, 0.9), 0 15px 28px rgba(0, 0, 0, 0.55), inset 0 2px 0 rgba(255, 255, 255, 0.18), 0 0 18px color-mix(in srgb, var(--control-accent) 35%, transparent);
	font-size: clamp(11px, 1.15vw, 18px);
	letter-spacing: 0.04em;
	transform: perspective(520px) rotateX(10deg);
}
.variant-console:hover,
.variant-console:focus-visible {
	transform: perspective(520px) rotateX(7deg) translateY(-4px);
}
.variant-console:active {
	transform: perspective(520px) rotateX(13deg) translateY(5px) scale(0.975);
}
@keyframes control-border-travel { to { transform: rotate(1turn); } }
@keyframes control-activate {
	0% { transform: scale(0); opacity: 0.9; }
	100% { transform: scale(1); opacity: 0; }
}
@media (prefers-reduced-motion: reduce) {
	.viewscreen-button::before { animation: none !important; }
}
</style>
