(() => {
  const initialize = () => {
    const element = document.querySelector("#pagefind");
    if (!element) return;

    const status = element.querySelector('[role="status"]');
    if (typeof window.PagefindUI !== "function") {
      if (status) status.textContent = "Search is unavailable. Browse the indexes below.";
      return;
    }

    const prefix = element.dataset.basePath || "";
    try {
      new window.PagefindUI({
        element: "#pagefind",
        bundlePath: `${prefix}/pagefind/`,
        baseUrl: `${prefix}/`,
        showSubResults: true,
        showImages: false,
      });
      if (status) status.remove();
    } catch {
      if (status) status.textContent = "Search could not be started. Browse the indexes below.";
    }
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initialize, { once: true });
  } else {
    initialize();
  }
})();
