(() => {
  const sidebar = document.querySelector("#nd-sidebar");
  const toggle = document.querySelector(".sidebar-toggle");
  const article = document.querySelector("#nd-page");
  const toc = document.querySelector("#nd-toc");

  const setSidebarOpen = (open) => {
    if (!sidebar || !toggle) return;
    sidebar.classList.toggle("is-open", open);
    toggle.setAttribute("aria-expanded", String(open));
  };

  if (sidebar && toggle) {
    toggle.addEventListener("click", () => {
      setSidebarOpen(toggle.getAttribute("aria-expanded") !== "true");
    });
    sidebar.addEventListener("click", (event) => {
      if (event.target.closest("a")) setSidebarOpen(false);
    });
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && toggle.getAttribute("aria-expanded") === "true") {
        const focusWasInSidebar = sidebar.contains(document.activeElement);
        setSidebarOpen(false);
        if (focusWasInSidebar) toggle.focus();
      }
    });
    document.addEventListener("click", (event) => {
      if (!sidebar.contains(event.target) && !toggle.contains(event.target)) {
        setSidebarOpen(false);
      }
    });
  }

  if (sidebar) {
    const currentPath = window.location.pathname.replace(/\/$/, "") || "/";
    for (const link of sidebar.querySelectorAll("a[href]")) {
      try {
        if (link.origin !== window.location.origin) continue;
        const destination = link.pathname.replace(/\/$/, "") || "/";
        if (destination === currentPath) link.setAttribute("aria-current", "page");
      } catch {
        // An invalid or external navigation target must not stop page setup.
      }
    }
  }

  if (article && toc) {
    const existingIds = new Set(
      Array.from(document.querySelectorAll("[id]"), (element) => element.id),
    );
    const headings = Array.from(article.querySelectorAll("h2, h3, h4")).filter(
      (heading) => !heading.closest(".not-prose, [data-toc-exclude]"),
    );
    const list = toc.querySelector("ol");
    const observed = [];

    for (const heading of headings) {
      if (!heading.id) {
        const base = heading.textContent
          .trim()
          .toLowerCase()
          .replace(/[^\p{L}\p{N}]+/gu, "-")
          .replace(/^-|-$/g, "") || "section";
        let candidate = base;
        let suffix = 2;
        while (existingIds.has(candidate)) {
          candidate = `${base}-${suffix}`;
          suffix += 1;
        }
        heading.id = candidate;
        existingIds.add(candidate);
      }

      const item = document.createElement("li");
      item.dataset.level = heading.tagName.slice(1);
      const link = document.createElement("a");
      link.href = `#${heading.id}`;
      link.textContent = heading.textContent.trim();
      item.append(link);
      list?.append(item);
      observed.push({ heading, link });
    }

    if (observed.length === 0) toc.hidden = true;

    let target = window.location.hash.slice(1);
    try {
      target = decodeURIComponent(target);
    } catch {
      target = "";
    }
    const initialHeading = observed.find(({ heading }) => heading.id === target)?.heading ?? observed[0]?.heading;
    const markActive = (activeHeadings) => {
      const active = new Set(activeHeadings);
      for (const { heading, link } of observed) {
        if (active.has(heading)) link.setAttribute("aria-current", "location");
        else link.removeAttribute("aria-current");
      }
      const activeRows = observed
        .filter(({ heading }) => active.has(heading))
        .map(({ link }) => link.parentElement)
        .filter(Boolean);
      const first = activeRows[0];
      const last = activeRows.at(-1);
      if (first && last && list) {
        const listTop = list.getBoundingClientRect().top;
        const top = first.getBoundingClientRect().top - listTop;
        const bottom = last.getBoundingClientRect().bottom - listTop;
        list.style.setProperty("--toc-active-top", `${top}px`);
        list.style.setProperty("--toc-active-height", `${bottom - top}px`);
      }
    };
    if (initialHeading) markActive([initialHeading]);

    if ("IntersectionObserver" in window && observed.length > 0) {
      const intersections = new Map();
      const observer = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            intersections.set(entry.target, entry.isIntersecting && entry.intersectionRatio >= 0.9);
          }
          let activeHeadings = observed
            .filter(({ heading }) => intersections.get(heading))
            .map(({ heading }) => heading);
          if (activeHeadings.length === 0) {
            const rootTop = entries.find((entry) => entry.rootBounds)?.rootBounds.top ?? 0;
            const nearest = observed
              .map(({ heading }) => ({ heading, distance: Math.abs(heading.getBoundingClientRect().top - rootTop) }))
              .sort((left, right) => left.distance - right.distance)[0];
            if (nearest) activeHeadings = [nearest.heading];
          }
          markActive(activeHeadings);
        },
        { root: null, threshold: 0.9 },
      );
      for (const { heading } of observed) observer.observe(heading);
    }
  }

  for (const button of document.querySelectorAll("[data-copy-markdown]")) {
    const header = button.closest(".document-header");
    const source = header?.querySelector("[data-markdown-source]");
    const status = header?.querySelector(".document-copy-status");
    if (!source || !status) continue;
    button.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(source.value);
        status.textContent = "Markdown copied to clipboard.";
      } catch {
        status.textContent = "Clipboard access is unavailable. Select the Markdown text to copy it.";
        source.hidden = false;
        source.focus();
        source.select();
      }
    });
  }

  for (const block of document.querySelectorAll("pre")) {
    const code = block.querySelector("code");
    if (!code) continue;
    const button = document.createElement("button");
    button.className = "copy-code";
    button.type = "button";
    button.textContent = "Copy";
    button.setAttribute("aria-label", "Copy code to clipboard");
    const status = document.createElement("p");
    status.className = "copy-code-status";
    status.setAttribute("role", "status");
    button.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(code.textContent);
        button.textContent = "Copied";
        status.textContent = "Code copied to clipboard.";
      } catch {
        button.textContent = "Copy unavailable";
        status.textContent = "Clipboard access is unavailable. Select the code to copy it.";
      }
    });
    block.append(button);
    block.after(status);
  }
})();
