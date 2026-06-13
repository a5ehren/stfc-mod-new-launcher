export function formatError(error: unknown): string {
	if (error instanceof Error) {
		return error.stack ?? error.message;
	}

	if (typeof error === "string") {
		return error;
	}

	if (error && typeof error === "object") {
		const record = error as Record<string, unknown>;
		const kind = typeof record.kind === "string" ? record.kind : "";
		const message =
			typeof record.message === "string"
				? record.message
				: typeof record.error === "string"
					? record.error
					: "";

		if (kind && message) {
			return `${kind}: ${message}`;
		}
		if (message) {
			return message;
		}
		if (kind) {
			return kind;
		}

		try {
			return JSON.stringify(error);
		} catch {
			return Object.prototype.toString.call(error);
		}
	}

	return String(error);
}
