<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, onMounted } from "vue";
import DevOverlay from "@/components/DevOverlay.vue";
import { setBootstrapStatus } from "@/lib/bootstrap-status";
import ConfigEditor from "@/views/ConfigEditor.vue";
import MainLauncher from "@/views/MainLauncher.vue";
import "@/styles/lcars.css";

const windowLabel = getCurrentWindow().label;
const activeView = computed(() =>
	windowLabel === "config-editor" ? ConfigEditor : MainLauncher,
);

onMounted(() => {
	setBootstrapStatus("Launcher ready");
});
</script>

<template>
  <DevOverlay />
  <component :is="activeView" />
</template>
