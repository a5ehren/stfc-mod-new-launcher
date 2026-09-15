<script setup lang="ts">
import { onMounted } from "vue";
import DevOverlay from "@/components/DevOverlay.vue";
import WindowChrome from "@/components/WindowChrome.vue";
import { setBootstrapStatus } from "@/lib/bootstrap-status";
import MainLauncher from "@/views/MainLauncher.vue";
import "@/styles/lcars.css";

onMounted(() => {
	setBootstrapStatus("Launcher ready");
});
</script>

<template>
  <WindowChrome />
  <DevOverlay />
  <MainLauncher />
</template>
