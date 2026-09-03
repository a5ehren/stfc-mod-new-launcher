<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onBeforeUnmount, onMounted } from "vue";

const appWindow = getCurrentWindow();
type ResizeDirection = Parameters<typeof appWindow.startResizeDragging>[0];
const resizeDirection = {
	north: "North",
	east: "East",
	south: "South",
	west: "West",
	northWest: "NorthWest",
	northEast: "NorthEast",
	southEast: "SouthEast",
	southWest: "SouthWest",
} as const satisfies Record<string, ResizeDirection>;
const interactiveSelector = [
	"a",
	"button",
	"input",
	"select",
	"textarea",
	"iframe",
	"[contenteditable]",
	"[role='button']",
	"[data-no-window-drag]",
].join(",");

function isInteractive(target: EventTarget | null) {
	return (
		target instanceof Element && target.closest(interactiveSelector) !== null
	);
}

function dragWindow(event: PointerEvent) {
	if (event.button === 0 && !isInteractive(event.target)) {
		void appWindow.startDragging();
	}
}

function toggleMaximize() {
	void appWindow.toggleMaximize();
}

function resize(direction: ResizeDirection, event: PointerEvent) {
	event.stopPropagation();
	if (event.button === 0) void appWindow.startResizeDragging(direction);
}

onMounted(() => document.addEventListener("pointerdown", dragWindow));
onBeforeUnmount(() => document.removeEventListener("pointerdown", dragWindow));
</script>

<template>
  <div class="window-controls" data-no-window-drag aria-label="Window controls">
    <button type="button" aria-label="Minimize window" title="Minimize" @click="appWindow.minimize()">
      <span class="minimize-icon" aria-hidden="true"></span>
    </button>
    <button type="button" aria-label="Maximize window" title="Maximize" @click="toggleMaximize">
      <span class="maximize-icon" aria-hidden="true"></span>
    </button>
    <button class="close" type="button" aria-label="Close window" title="Close" @click="appWindow.close()">
      <span class="close-icon" aria-hidden="true"></span>
    </button>
  </div>

  <div class="resize-edge top" data-no-window-drag @pointerdown="resize(resizeDirection.north, $event)"></div>
  <div class="resize-edge right" data-no-window-drag @pointerdown="resize(resizeDirection.east, $event)"></div>
  <div class="resize-edge bottom" data-no-window-drag @pointerdown="resize(resizeDirection.south, $event)"></div>
  <div class="resize-edge left" data-no-window-drag @pointerdown="resize(resizeDirection.west, $event)"></div>
  <div class="resize-corner top-left" data-no-window-drag @pointerdown="resize(resizeDirection.northWest, $event)"></div>
  <div class="resize-corner top-right" data-no-window-drag @pointerdown="resize(resizeDirection.northEast, $event)"></div>
  <div class="resize-corner bottom-right" data-no-window-drag @pointerdown="resize(resizeDirection.southEast, $event)"></div>
  <div class="resize-corner bottom-left" data-no-window-drag @pointerdown="resize(resizeDirection.southWest, $event)"></div>
</template>

<style scoped>
.window-controls {
	position: fixed;
	top: 0;
	right: 0;
	z-index: 10001;
	display: flex;
}
.window-controls button {
	display: grid;
	place-items: center;
	width: 46px;
	height: 32px;
	padding: 0;
	border: 0;
	background: rgba(1, 9, 22, 0.72);
	color: #d8f5ff;
	cursor: pointer;
	transition: background 120ms ease, color 120ms ease;
}
.window-controls button:hover,
.window-controls button:focus-visible { background: rgba(87, 201, 255, 0.25); outline: none; }
.window-controls .close:hover,
.window-controls .close:focus-visible { background: #c42b3b; color: white; }
.minimize-icon { width: 11px; border-top: 1px solid currentColor; }
.maximize-icon { width: 10px; height: 9px; border: 1px solid currentColor; box-sizing: border-box; }
.close-icon { position: relative; width: 12px; height: 12px; }
.close-icon::before,
.close-icon::after { content: ""; position: absolute; top: 5px; left: 0; width: 12px; border-top: 1px solid currentColor; transform: rotate(45deg); }
.close-icon::after { transform: rotate(-45deg); }
.resize-edge,
.resize-corner { position: fixed; z-index: 10002; }
.resize-edge.top { top: 0; left: 8px; right: 138px; height: 5px; cursor: ns-resize; }
.resize-edge.right { top: 8px; right: 0; bottom: 8px; width: 5px; cursor: ew-resize; }
.resize-edge.bottom { right: 8px; bottom: 0; left: 8px; height: 5px; cursor: ns-resize; }
.resize-edge.left { top: 8px; bottom: 8px; left: 0; width: 5px; cursor: ew-resize; }
.resize-corner { width: 10px; height: 10px; }
.top-left { top: 0; left: 0; cursor: nwse-resize; }
.top-right { top: 0; right: 0; cursor: nesw-resize; }
.bottom-right { right: 0; bottom: 0; cursor: nwse-resize; }
.bottom-left { bottom: 0; left: 0; cursor: nesw-resize; }
</style>
