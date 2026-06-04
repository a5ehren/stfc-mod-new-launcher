<template>
  <main class="launcher-shell" aria-labelledby="app-title">
    <h1 id="app-title">STFC Community Mod</h1>
    <p class="status">Launcher shell</p>
  </main>
</template>

<style scoped>
.launcher-shell {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding: 2rem;
  text-align: center;
}

h1 {
  margin: 0;
  font-size: 2rem;
  font-weight: 700;
}

.status {
  margin: 0;
  color: #5f6f7a;
  font-size: 1rem;
}
</style>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;
  color: #0f0f0f;
  background-color: #f6f6f6;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

body {
  margin: 0;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #1d2429;
  }

  .status {
    color: #9fb1bd;
  }
}
</style>
