<script setup lang="ts">
defineProps<{ message: string; warning?: string }>();
</script>

<template>
  <div class="status-strip">
    <span>{{ message }}</span>
    <strong v-if="warning">{{ warning }}</strong>
  </div>
</template>

<style scoped>
.status-strip {
  min-height: 28px;
  color: var(--lcars-tan);
  font-size: 18px;
  display: flex;
  gap: 18px;
  align-items: center;
}
strong {
  color: var(--lcars-gold);
}
</style>