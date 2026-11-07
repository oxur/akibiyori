(() => {
  "use strict";

  const root = document.documentElement;
  const key = root.dataset.akiModeKey || "";
  const rawDefault = root.dataset.akiDefaultMode;
  const validPreference = (value) => value === "dark" || value === "light" || value === "system";
  const authorDefault = validPreference(rawDefault) ? rawDefault : "dark";
  const picker = () => document.querySelector("[data-aki-mode-picker]");
  let preference = authorDefault;
  let media = null;
  let bound = false;
  let restoreTask = null;

  const readPreference = () => {
    if (!key) return authorDefault;
    try {
      const stored = window.localStorage.getItem(key);
      return validPreference(stored) ? stored : authorDefault;
    } catch {
      return preference;
    }
  };

  const effectiveMode = () => {
    if (preference !== "system") return preference;
    try {
      media ??= window.matchMedia("(prefers-color-scheme: dark)");
      return media.matches ? "dark" : "light";
    } catch {
      return "dark";
    }
  };

  const apply = () => {
    const mode = effectiveMode();
    root.classList.toggle("dark", mode === "dark");
    root.classList.toggle("light", mode === "light");
    root.dataset.akiMode = mode;
    const control = picker();
    if (control) control.value = preference;
  };

  const persist = (value) => {
    if (!key) return;
    try {
      window.localStorage.setItem(key, value);
    } catch {
      // Keep the selected mode for this page; persistence is unavailable.
    }
  };

  const cancelRestore = () => {
    if (restoreTask !== null) window.clearTimeout(restoreTask);
    restoreTask = null;
  };

  const refreshFromPreference = () => {
    preference = readPreference();
    apply();
  };

  const scheduleRestore = () => {
    cancelRestore();
    restoreTask = window.setTimeout(() => {
      restoreTask = null;
      refreshFromPreference();
    }, 0);
  };

  preference = readPreference();
  apply();

  try {
    media ??= window.matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", () => {
      if (preference === "system") apply();
    });
  } catch {
    media = null;
  }

  const initialize = () => {
    if (bound) return;
    const control = picker();
    if (!control) return;
    bound = true;
    control.value = preference;
    control.addEventListener("change", () => {
      const selected = control.value;
      if (!validPreference(selected)) return;
      cancelRestore();
      preference = selected;
      apply();
      persist(preference);
    });
    control.closest(".mode-control")?.removeAttribute("hidden");
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initialize, { once: true });
  } else {
    initialize();
  }

  window.addEventListener("pageshow", () => {
    refreshFromPreference();
    scheduleRestore();
  });
  window.addEventListener("popstate", scheduleRestore);
})();
