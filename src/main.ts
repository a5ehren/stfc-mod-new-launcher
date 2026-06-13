import { createApp } from "vue";
import {
	clearBootstrapError,
	setBootstrapError,
	setBootstrapStatus,
} from "@/lib/bootstrap-status";
import { formatError } from "@/lib/formatError";
import App from "./App.vue";

setBootstrapStatus("Starting launcher");
clearBootstrapError();

const app = createApp(App);

app.config.errorHandler = (error, _instance, info) => {
	setBootstrapError(`Vue error while ${info}\n\n${formatError(error)}`);
};

window.addEventListener("error", (event) => {
	setBootstrapError(
		`Unhandled window error\n\n${formatError(event.error ?? event.message)}`,
	);
});

window.addEventListener("unhandledrejection", (event) => {
	setBootstrapError(
		`Unhandled promise rejection\n\n${formatError(event.reason)}`,
	);
});

app.mount("#app");
