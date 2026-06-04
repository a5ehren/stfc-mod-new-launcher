import { mount } from "@vue/test-utils";
import { describe, expect, test } from "vitest";
import App from "./App.vue";

describe("App", () => {
  test("does not render the stale greet command form", () => {
    const wrapper = mount(App);

    expect(wrapper.find("#greet-input").exists()).toBe(false);
    expect(wrapper.text()).not.toContain("Greet");
  });
});
