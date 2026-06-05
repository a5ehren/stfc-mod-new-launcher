<script setup lang="ts">
withDefaults(
	defineProps<{
		bannerText?: string;
		compactHeader?: boolean;
	}>(),
	{
		bannerText: "LCARS ACCESS 333",
		compactHeader: false,
	},
);
</script>

<template>
  <section class="lcars-shell" :class="{ 'compact-header': compactHeader }">
    <aside class="left-frame">
      <div class="panel panel-top">LCARS</div>
      <div class="panel panel-fill">02-262000</div>
      <div class="panel panel-bottom">10-31</div>
    </aside>
    <main class="content-frame">
      <header class="top-frame">
        <div class="banner">{{ bannerText }}</div>
        <slot name="cascade" />
      </header>
      <div class="bar-panel">
        <div class="bar blue"></div>
        <div class="bar orange"></div>
        <div class="bar violet"></div>
        <div class="bar red"></div>
      </div>
      <section class="content">
        <slot />
      </section>
    </main>
  </section>
</template>

<style scoped>
.lcars-shell {
  display: grid;
  grid-template-columns: 164px 1fr;
  gap: 8px;
  width: 100vw;
  height: 100vh;
  padding: 10px;
  box-sizing: border-box;
  background: #000;
}
.left-frame {
  display: grid;
  grid-template-rows: 90px 1fr 70px;
  color: #000;
  font-weight: 700;
  text-align: right;
}
.panel {
  padding: 12px;
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
}
.panel-top {
  background: var(--lcars-blue);
  border-radius: var(--lcars-radius) 0 0 0;
}
.panel-fill {
  background: var(--lcars-red);
  border-top: 8px solid #000;
  border-bottom: 8px solid #000;
}
.panel-bottom {
  background: var(--lcars-orange);
  border-radius: 0 0 0 var(--lcars-radius);
}
.content-frame {
  display: grid;
  grid-template-rows: 156px 24px 1fr;
  min-width: 0;
  min-height: 0;
}
.compact-header .content-frame {
  grid-template-rows: 66px 24px 1fr;
}
.top-frame {
  display: grid;
  align-items: end;
}
.compact-header .top-frame {
  align-items: start;
}
.banner {
  color: var(--lcars-orange);
  font-size: 42px;
  line-height: 1;
  text-align: right;
}
.bar-panel {
  display: grid;
  grid-template-columns: 4fr 1fr 2fr 1fr;
  gap: 8px;
}
.bar { height: 24px; }
.blue { background: var(--lcars-blue); }
.orange { background: var(--lcars-orange); }
.violet { background: var(--lcars-violet); }
.red { background: var(--lcars-red); }
.content {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  padding: 22px 0 0 22px;
}
.compact-header .content {
  padding-top: 8px;
}
</style>
