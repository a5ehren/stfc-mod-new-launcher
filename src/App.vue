<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed } from "vue";
import ConfigEditor from "@/views/ConfigEditor.vue";
import MainLauncher from "@/views/MainLauncher.vue";
import "@/styles/lcars.css";

const windowLabel = getCurrentWindow().label;
const _activeView = computed(() =>
	windowLabel === "config-editor" ? ConfigEditor : MainLauncher,
);
</script>

<template>
  <component :is="activeView" />
</template>