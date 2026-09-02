<template>
  <svg class="viewscreen-frame" viewBox="0 0 1511 688" preserveAspectRatio="none" aria-hidden="true">
    <defs>
      <filter id="viewscreen-glow" x="-20%" y="-20%" width="140%" height="140%">
        <feGaussianBlur stdDeviation="7" result="blur" />
        <feMerge>
          <feMergeNode in="blur" />
          <feMergeNode in="SourceGraphic" />
        </feMerge>
      </filter>
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
	opacity: 0.22;
	filter: url(#viewscreen-glow);
}
.pulse {
	stroke: #bcecff;
	stroke-width: 7;
	stroke-linecap: round;
	stroke-dasharray: 70 1250;
	filter: url(#viewscreen-glow);
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
	filter: url(#viewscreen-glow);
	animation: corner-breathe 2.8s ease-in-out infinite alternate;
}
.corner-b { animation-delay: -0.7s; }
.corner-c { animation-delay: -1.4s; }
.corner-d { animation-delay: -2.1s; }
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
