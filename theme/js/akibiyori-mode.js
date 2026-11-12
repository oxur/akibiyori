(() => {
  "use strict";

  const root = document.documentElement;
  const presentation = root.dataset.akiPresentation === "demo" ? "demo" : "deploy";
  const modeKey = root.dataset.akiModeKey || "";
  const themeKey = root.dataset.akiThemeKey || "";
  const rawDefaultMode = root.dataset.akiDefaultMode;
  const validPreference = (value) => value === "dark" || value === "light" || value === "system";
  const authorMode = validPreference(rawDefaultMode) ? rawDefaultMode : "dark";
  const authorTheme = root.dataset.akiDefaultVariant || "";
  const profiles = new Map(Array.from(document.querySelectorAll('meta[name="aki-profile"]'), (meta) => [meta.dataset.id, meta.dataset]));
  const modePicker = () => document.querySelector("[data-aki-mode-picker]");
  const themePicker = () => document.querySelector("[data-aki-theme-picker]");
  let preference = authorMode;
  let selectedTheme = authorTheme;
  let media = null;
  let bound = false;
  let restoreTask = null;

  const readModePreference = () => {
    if (!modeKey) return authorMode;
    try {
      const stored = window.localStorage.getItem(modeKey);
      return validPreference(stored) ? stored : authorMode;
    } catch {
      return preference;
    }
  };

  const readThemePreference = () => {
    if (!themeKey) return authorTheme;
    try {
      const stored = window.localStorage.getItem(themeKey);
      return profiles.has(stored) ? stored : authorTheme;
    } catch {
      return selectedTheme;
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

  const applyProfile = (profile) => {
    if (!profile) return;
    const mode = profile.mode === "light" ? "light" : "dark";
    root.classList.toggle("dark", mode === "dark");
    root.classList.toggle("light", mode === "light");
    root.dataset.akiMode = mode;
    root.dataset.akiVariant = profile.id;
    root.dataset.akiTheme = profile.family;

    let author = document.getElementById("aki-author-theme");
    if (profile.stylesheet) {
      if (!author) {
        author = document.createElement("link");
        author.id = "aki-author-theme";
        author.rel = "stylesheet";
        document.head.append(author);
      }
      if (author.href !== new URL(profile.stylesheet, document.baseURI).href) {
        author.href = profile.stylesheet;
      }
    } else {
      author?.remove();
    }
    root.dispatchEvent(new CustomEvent("akibiyori:profile-change", {
      detail: { profile: profile.id },
    }));
  };

  const applyDeploy = () => {
    const mode = effectiveMode();
    const variant = mode === "dark" ? root.dataset.akiDarkVariant : root.dataset.akiLightVariant;
    applyProfile(profiles.get(variant));
    const control = modePicker();
    if (control) control.value = preference;
  };

  const urlTheme = () => {
    const values = new URLSearchParams(window.location.search).getAll("theme");
    return values.length === 1 && profiles.has(values[0]) ? values[0] : null;
  };

  const persist = (key, value) => {
    if (!key) return;
    try {
      window.localStorage.setItem(key, value);
    } catch {
      // Keep the selection for this page; persistence is unavailable.
    }
  };

  const applyDemo = () => {
    const selected = profiles.get(selectedTheme) || profiles.get(authorTheme);
    if (!selected) return;
    selectedTheme = selected.id;
    applyProfile(selected);
    const control = themePicker();
    if (control) control.value = selectedTheme;
  };

  const refreshFromPreference = () => {
    if (presentation === "demo") {
      const fromUrl = urlTheme();
      selectedTheme = fromUrl || readThemePreference();
      if (fromUrl) persist(themeKey, fromUrl);
      applyDemo();
      return;
    }
    preference = readModePreference();
    applyDeploy();
  };

  const cancelRestore = () => {
    if (restoreTask !== null) window.clearTimeout(restoreTask);
    restoreTask = null;
  };

  const scheduleRestore = () => {
    cancelRestore();
    restoreTask = window.setTimeout(() => {
      restoreTask = null;
      refreshFromPreference();
    }, 0);
  };

  refreshFromPreference();

  if (presentation === "deploy") {
    try {
      media ??= window.matchMedia("(prefers-color-scheme: dark)");
      media.addEventListener("change", () => {
        if (preference === "system") applyDeploy();
      });
    } catch {
      media = null;
    }
  }

  const initialize = () => {
    if (bound) return;
    const control = presentation === "demo" ? themePicker() : modePicker();
    if (!control) return;
    bound = true;
    if (presentation === "demo") {
      control.value = selectedTheme;
      control.addEventListener("change", () => {
        const selected = control.value;
        if (!profiles.has(selected)) return;
        cancelRestore();
        selectedTheme = selected;
        const url = new URL(window.location.href);
        url.searchParams.set("theme", selected);
        window.history.replaceState(window.history.state, "", `${url.pathname}${url.search}${url.hash}`);
        persist(themeKey, selectedTheme);
        applyDemo();
      });
    } else {
      control.value = preference;
      control.addEventListener("change", () => {
        const selected = control.value;
        if (!validPreference(selected)) return;
        cancelRestore();
        preference = selected;
        applyDeploy();
        persist(modeKey, preference);
      });
    }
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
