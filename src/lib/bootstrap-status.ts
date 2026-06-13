import { ref } from "vue";

const defaultStatus = "Starting launcher";

const bootstrapStatus = ref(defaultStatus);
const bootstrapError = ref<string | null>(null);

export function setBootstrapStatus(status: string) {
	bootstrapStatus.value = status;
}

export function resetBootstrapStatus() {
	bootstrapStatus.value = defaultStatus;
}

export function setBootstrapError(error: unknown) {
	if (error instanceof Error) {
		bootstrapError.value = error.stack ?? error.message;
		return;
	}
	bootstrapError.value = String(error);
}

export function clearBootstrapError() {
	bootstrapError.value = null;
}

export function useBootstrapStatus() {
	return {
		bootstrapStatus,
		bootstrapError,
	};
}
