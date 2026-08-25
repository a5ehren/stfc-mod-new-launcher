<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import LcarsButton from "@/components/lcars/LcarsButton.vue";
import LcarsShell from "@/components/lcars/LcarsShell.vue";
import { miProvision, miWizardPlan, onProgress } from "@/lib/commands";
import { formatError } from "@/lib/formatError";
import {
	INSTANCE_NAME_PATTERN,
	initialWizardState,
	nextStep,
	type WizardState,
} from "@/lib/multiInstanceWizard";
import type { WizardPlanDto } from "@/types/launcher";

const emit = defineEmits<{ done: [] }>();

const plan = ref<WizardPlanDto | null>(null);
const state = ref<WizardState | null>(null);
const progressMessage = ref("");
const error = ref("");
const provisioning = ref(false);
const newName = ref("");
let unlistenProgress: (() => void) | null = null;

const names = computed(() => state.value?.names ?? []);
const invalidName = computed(() => {
	const n = newName.value.trim();
	return n !== "" && !INSTANCE_NAME_PATTERN.test(n);
});
const provisionDisabled = computed(
	() => provisioning.value || names.value.length === 0,
);

function advance() {
	if (state.value) state.value = nextStep(state.value);
}

function addName() {
	if (!state.value) return;
	const n = newName.value.trim();
	if (!INSTANCE_NAME_PATTERN.test(n) || state.value.names.includes(n)) return;
	state.value = { ...state.value, names: [...state.value.names, n] };
	newName.value = "";
}

function removeName(name: string) {
	if (!state.value) return;
	state.value = {
		...state.value,
		names: state.value.names.filter((n) => n !== name),
	};
}

async function provision() {
	if (!state.value) return;
	error.value = "";
	provisioning.value = true;
	try {
		await miProvision(names.value);
		state.value = nextStep(state.value);
	} catch (e) {
		error.value = formatError(e);
	} finally {
		provisioning.value = false;
	}
}

onMounted(async () => {
	unlistenProgress = await onProgress((event) => {
		if (event.operation === "mi_provision")
			progressMessage.value = event.message;
	});
	try {
		plan.value = await miWizardPlan();
		state.value = initialWizardState(plan.value);
	} catch (e) {
		error.value = formatError(e);
	}
});

onBeforeUnmount(() => {
	unlistenProgress?.();
	unlistenProgress = null;
});
</script>

<template>
  <LcarsShell banner-text="Multi-Instance Setup" compact-header>
    <div class="wizard">
      <p v-if="error" class="error">{{ error }}</p>

      <section v-if="state?.step === 'warnings'" class="step">
        <h2>Before you begin</h2>
        <p>
          Each game instance runs as a separate hidden macOS/Windows user. A new
          instance starts with a NEW guest game account. If that user is deleted
          or lost without a backup or a linked Scopely ID, the account is
          permanently unrecoverable.
        </p>
        <p>
          Multiboxing is subject to Scopely's Terms of Service. Use at your own
          risk.
        </p>
        <label class="acknowledge">
          <input
            type="checkbox"
            :checked="state.acknowledged"
            @change="state = { ...state!, acknowledged: ($event.target as HTMLInputElement).checked }"
          />
          I understand new instances are unrecoverable guest accounts until backed up
        </label>
        <div class="step-actions">
          <LcarsButton tone="orange" edge="single" :disabled="!state.acknowledged" @click="advance">
            Continue
          </LcarsButton>
          <LcarsButton tone="red" edge="single" @click="emit('done')">Cancel</LcarsButton>
        </div>
      </section>

      <section v-else-if="state?.step === 'relocate'" class="step">
        <h2>Move the game to a shared location</h2>
        <p>
          So every instance can read the same install, the game will be moved:
        </p>
        <p class="paths">{{ plan?.gameSource }} &rarr; {{ plan?.sharedRoot }}</p>
        <p>
          This happens once, with an administrator prompt, when you provision.
          The launcher and all instances will use the shared copy from then on.
        </p>
        <div class="step-actions">
          <LcarsButton tone="blue" edge="single" @click="advance">Continue</LcarsButton>
          <LcarsButton tone="red" edge="single" @click="emit('done')">Cancel</LcarsButton>
        </div>
      </section>

      <section v-else-if="state?.step === 'instances'" class="step">
        <h2>Create instances</h2>
        <p>Name each extra instance (lowercase letters, digits, dashes).</p>
        <div class="name-entry">
          <input
            v-model="newName"
            class="name-input"
            :class="{ invalid: invalidName }"
            placeholder="alt2"
            maxlength="16"
            :disabled="provisioning"
            @keydown.enter="addName"
          />
          <LcarsButton tone="tan" edge="single" :disabled="!newName.trim() || invalidName || provisioning" @click="addName">
            Add
          </LcarsButton>
        </div>
        <p v-if="invalidName" class="error">Names must be 1-16 chars of a-z, 0-9, -</p>
        <ul class="name-list">
          <li v-for="name in names" :key="name">
            <span>stfc-{{ name }}</span>
            <button class="remove" :disabled="provisioning" @click="removeName(name)">remove</button>
          </li>
        </ul>
        <p v-if="progressMessage" class="progress">{{ progressMessage }}</p>
        <div class="step-actions">
          <LcarsButton tone="orange" edge="single" :disabled="provisionDisabled" @click="provision">
            {{ provisioning ? "Provisioning…" : "Provision" }}
          </LcarsButton>
          <LcarsButton tone="red" edge="single" :disabled="provisioning" @click="emit('done')">Cancel</LcarsButton>
        </div>
      </section>

      <section v-else-if="state?.step === 'done'" class="step">
        <h2>Multi-instance mode enabled</h2>
        <p>
          {{ names.length }} instance{{ names.length === 1 ? "" : "s" }} provisioned.
          Start, stop, and back them up from the Instances panel on the main screen.
        </p>
        <p>
          Remember: back up each instance after first login — a guest account
          without a backup or Scopely ID link cannot be recovered.
        </p>
        <LcarsButton tone="gold" edge="single" @click="emit('done')">Done</LcarsButton>
      </section>
    </div>
  </LcarsShell>
</template>

<style scoped>
.wizard {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  height: 100%;
  padding: 24px 48px;
  color: #fff;
  gap: 12px;
}
.step {
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-width: 640px;
}
.step h2 {
  margin: 0;
  color: var(--lcars-orange);
  text-transform: uppercase;
}
.step-actions {
  display: flex;
  gap: 8px;
}
.acknowledge {
  display: flex;
  align-items: center;
  gap: 8px;
}
.paths {
  font-family: monospace;
  color: var(--lcars-blue);
}
.error {
  color: var(--lcars-red);
}
.progress {
  color: var(--lcars-gold);
}
.name-entry {
  display: flex;
  gap: 8px;
  align-items: center;
}
.name-input {
  background: #000;
  border: 2px solid var(--lcars-blue);
  border-radius: 8px;
  color: #fff;
  padding: 8px 12px;
  font-size: 16px;
}
.name-input.invalid {
  border-color: var(--lcars-red);
}
.name-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.name-list li {
  display: flex;
  justify-content: space-between;
  gap: 24px;
}
.remove {
  background: none;
  border: 0;
  color: var(--lcars-red);
  cursor: pointer;
  text-transform: uppercase;
}
</style>
