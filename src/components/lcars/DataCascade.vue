<template>
  <div class="data-cascade" aria-hidden="true">
    <div v-for="column in 14" :key="column" class="data-column">
      <span v-for="row in 7" :key="row">{{ String(column * 137 + row * 29).padStart(5, "0") }}</span>
    </div>
  </div>
</template>

<style scoped>
.data-cascade {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  height: 96px;
  overflow: hidden;
  color: var(--lcars-orange);
  font-size: 16px;
  line-height: 1;
  opacity: 0.9;
}
.data-column {
  display: grid;
  gap: 2px;
  text-align: right;
}
</style>