<script setup lang="ts">
defineProps<{ targets: Record<string, Record<string, unknown>> }>();
</script>

<template>
  <section class="sync-targets">
    <h3>Sync Targets</h3>
    <p v-if="Object.keys(targets).length === 0">No sync targets configured.</p>
    <ul v-else>
      <li v-for="name in Object.keys(targets)" :key="name">{{ name }}</li>
    </ul>
  </section>
</template>

<style scoped>
.sync-targets {
  color: var(--lcars-tan);
}
h3 {
  color: var(--lcars-blue);
}
</style>