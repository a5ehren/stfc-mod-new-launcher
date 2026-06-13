import { ref } from "vue";
import { formatError } from "@/lib/formatError";

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
	bootstrapError.value = formatError(error);
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
