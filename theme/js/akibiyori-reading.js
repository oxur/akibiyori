(() => {
  const clearPropertyFilter = (filter) => {
    filter.value = "";
    filter.dispatchEvent(new Event("input", { bubbles: true }));
  };
  const resetPropertyFilters = (panel) => {
    for (const filter of panel.querySelectorAll("[data-property-filter]")) clearPropertyFilter(filter);
  };
  const panelActivators = new WeakMap();
  const callbackOwners = new WeakMap();
  const sidebar = document.querySelector("#nd-sidebar");
  const toggle = document.querySelector(".sidebar-toggle");
  const article = document.querySelector("#nd-page");
  const toc = document.querySelector("#nd-toc");
  const frame = document.querySelector("#fd-spacious-layout");
  const collapse = document.querySelector(".sidebar-collapse");
  const reopen = document.querySelector("#sidebar-reopen");

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
        if (sidebar.querySelector("[data-sidebar-sections][open]")) return;
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

    const sectionDetails = sidebar.querySelector("[data-sidebar-sections]");
    const sectionSummary = sectionDetails?.querySelector("summary");
    const sectionLabel = sectionSummary?.querySelector("[data-section-current-label]");
    const sectionIcon = sectionSummary?.querySelector("[data-section-current-icon]");
    const sectionLinks = Array.from(sidebar.querySelectorAll("[data-sidebar-section][href]"));
    const matches = sectionLinks
      .map((link) => {
        try {
          const url = new URL(link.href, window.location.href);
          if (url.origin !== window.location.origin) return null;
          const path = url.pathname.replace(/\/$/, "") || "/";
          const matchesPath = path === "/" || currentPath === path || currentPath.startsWith(`${path}/`);
          return matchesPath ? { link, length: path.length } : null;
        } catch {
          return null;
        }
      })
      .filter(Boolean)
      .sort((left, right) => right.length - left.length);
    const currentSection = matches[0]?.link;
    if (currentSection) {
      const label = currentSection.querySelector("[data-section-label]");
      const icon = currentSection.querySelector("[data-sidebar-link-icon]");
      if (label && sectionLabel) sectionLabel.textContent = label.textContent;
      if (icon && sectionIcon) {
        const replacement = icon.cloneNode(true);
        replacement.removeAttribute("data-sidebar-link-icon");
        replacement.className = "sidebar-section-summary-icon";
        sectionIcon.replaceWith(replacement);
        replacement.dataset.sectionCurrentIcon = "";
      }
      for (const link of sectionLinks) {
        if (link === currentSection) link.setAttribute("aria-current", "page");
        else link.removeAttribute("aria-current");
      }
    }

    if (sectionDetails && sectionSummary) {
      document.addEventListener("click", (event) => {
        if (!sectionDetails.contains(event.target)) sectionDetails.open = false;
      });
      document.addEventListener("keydown", (event) => {
        if (event.key === "Escape" && sectionDetails.open) {
          const focusWasInside = sectionDetails.contains(document.activeElement);
          sectionDetails.open = false;
          if (focusWasInside) sectionSummary.focus();
        }
      });
    }
  }

  if (sidebar && frame && collapse && reopen) {
    const mobile = window.matchMedia("(max-width: 48rem)");
    const setCollapsed = (collapsed, restoreFocus = false) => {
      if (mobile.matches) return;
      frame.dataset.sidebarCollapsed = String(collapsed);
      sidebar.dataset.collapsed = String(collapsed);
      sidebar.inert = collapsed;
      collapse.setAttribute("aria-expanded", String(!collapsed));
      reopen.hidden = !collapsed;
      reopen.setAttribute("aria-expanded", String(!collapsed));
      if (restoreFocus) (collapsed ? reopen : collapse).focus();
    };
    collapse.addEventListener("click", () => setCollapsed(true, true));
    reopen.addEventListener("click", () => setCollapsed(false, true));
    setCollapsed(false);
    const resetForMobile = () => {
      if (!mobile.matches) return;
      const focusWasInDesktopControls = collapse.contains(document.activeElement) || reopen.contains(document.activeElement);
      frame.dataset.sidebarCollapsed = "false";
      sidebar.dataset.collapsed = "false";
      sidebar.inert = false;
      collapse.setAttribute("aria-expanded", "true");
      reopen.hidden = true;
      if (focusWasInDesktopControls) toggle?.focus();
      setSidebarOpen(false);
    };
    mobile.addEventListener("change", resetForMobile);
    resetForMobile();
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
    const exampleCode = Boolean(block.closest("[data-code-panel]"));
    if (exampleCode) continue;
    button.className = exampleCode ? "copy-code example-copy-code" : "copy-code";
    button.type = "button";
    if (exampleCode) {
      button.innerHTML = '<svg class="example-copy-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2"></rect><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"></path></svg>';
    } else {
      button.textContent = "Copy";
    }
    button.setAttribute("aria-label", "Copy code to clipboard");
    const status = document.createElement("p");
    status.className = "copy-code-status";
    status.setAttribute("role", "status");
    status.hidden = true;
    button.addEventListener("click", async () => {
      status.hidden = false;
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

  for (const group of document.querySelectorAll("[data-media-group]")) {
    const select = group.querySelector("[data-media-select]");
    const panels = Array.from(group.querySelectorAll(":scope > [data-media-panel]"));
    if (!select || panels.length < 2) continue;
    panels.forEach((panel, index) => {
      const option = document.createElement("option");
      option.value = String(index);
      option.textContent = panel.dataset.mediaLabel || `Media type ${index + 1}`;
      select.append(option);
      panel.hidden = index !== 0;
    });
    select.disabled = false;
    const activate = (selectedIndex, { resetFilters = true } = {}) => {
      if (!panels[selectedIndex]) return;
      select.value = String(selectedIndex);
      panels.forEach((panel, index) => {
        panel.hidden = index !== selectedIndex;
        if (index === selectedIndex && resetFilters) resetPropertyFilters(panel);
      });
    };
    panels.forEach((panel, index) => panelActivators.set(panel, (options) => activate(index, options)));
    select.addEventListener("change", () => activate(Number(select.value)));
  }

  for (const group of document.querySelectorAll("[data-choice-group]")) {
    const select = group.querySelector("[data-choice-select]");
    const panels = Array.from(group.querySelectorAll(":scope > [data-choice-panel]"));
    if (!select || panels.length < 2) continue;
    panels.forEach((panel, index) => {
      const option = document.createElement("option");
      option.value = String(index);
      option.textContent = panel.dataset.choiceLabel || `Choice ${index + 1}`;
      select.append(option);
      panel.hidden = index !== 0;
    });
    select.disabled = false;
    const activate = (selectedIndex, { resetFilters = true } = {}) => {
      if (!panels[selectedIndex]) return;
      select.value = String(selectedIndex);
      panels.forEach((panel, index) => {
        panel.hidden = index !== selectedIndex;
        if (index === selectedIndex && resetFilters) resetPropertyFilters(panel);
      });
    };
    panels.forEach((panel, index) => panelActivators.set(panel, (options) => activate(index, options)));
    select.addEventListener("change", () => activate(Number(select.value)));
  }

  for (const group of document.querySelectorAll(".example-group")) {
    const select = group.querySelector("[data-example-select]");
    const panels = Array.from(group.querySelectorAll(":scope > [data-example-panel]"));
    if (panels.length === 0) continue;
    const origin = group.querySelector("[data-example-origin]");
    const notesSlot = group.querySelector("[data-example-notes-slot]");
    const languageControl = group.querySelector("[data-example-language-control]");
    const languageSelect = group.querySelector("[data-example-language]");
    const copyButton = group.querySelector("[data-example-copy]");
    const copyStatus = group.querySelector("[data-example-copy-status]");
    if (!languageControl || !languageSelect || !copyButton || !copyStatus) continue;

    const notesByPanel = panels.map((panel) =>
      panel.querySelector(":scope > [data-example-notes]"),
    );
    let selectedLanguage = null;
    let visibleCodePanels = [];
    const setOrigin = (panel) => {
      if (!origin) return;
      origin.textContent = panel.dataset.exampleUnavailable === "true"
        ? "Unavailable"
        : panel.dataset.exampleProvided === "true"
          ? "Provided"
          : "Derived";
    };
    const showLanguagePanel = (index) => {
      visibleCodePanels.forEach((codePanel, panelIndex) => {
        codePanel.hidden = panelIndex !== index;
      });
      const activeCode = visibleCodePanels[index]?.dataset.codeLabel;
      if (activeCode) selectedLanguage = activeCode;
      resetPropertyFilters(panels[Number(select?.value || 0)]);
    };
    const activateVariant = (index) => {
      const panel = panels[index];
      if (!panel) return;
      panels.forEach((candidate, candidateIndex) => {
        candidate.hidden = candidateIndex !== index;
      });
      setOrigin(panel);
      const notes = notesByPanel[index];
      if (notes && notesSlot) {
        const summary = notes.querySelector(":scope > summary");
        if (summary) summary.hidden = true;
        notes.open = true;
        notesSlot.replaceChildren(notes);
      } else if (notesSlot) {
        notesSlot.replaceChildren();
      }
      visibleCodePanels = Array.from(panel.querySelectorAll(":scope > [data-code-panel]"));
      languageSelect.replaceChildren();
      if (visibleCodePanels.length === 0) {
        languageControl.hidden = true;
        languageSelect.disabled = true;
        copyButton.hidden = true;
        copyButton.disabled = true;
        copyStatus.textContent = "";
        return;
      }
      for (const [codeIndex, codePanel] of visibleCodePanels.entries()) {
        const label = codePanel.dataset.codeLabel || `Code ${codeIndex + 1}`;
        const option = document.createElement("option");
        option.value = String(codeIndex);
        option.textContent = label;
        languageSelect.append(option);
      }
      const savedIndex = visibleCodePanels.findIndex(
        (codePanel) => codePanel.dataset.codeLabel === selectedLanguage,
      );
      const activeIndex = savedIndex >= 0 ? savedIndex : 0;
      languageSelect.value = String(activeIndex);
      languageControl.hidden = visibleCodePanels.length < 2;
      languageSelect.disabled = visibleCodePanels.length < 2;
      copyButton.hidden = false;
      copyButton.disabled = false;
      copyStatus.textContent = "";
      showLanguagePanel(activeIndex);
    };

    if (select && panels.length > 1) {
      const mediaCounts = new Map();
      for (const panel of panels) {
        const mediaType = panel.dataset.mediaType || "Example";
        mediaCounts.set(mediaType, (mediaCounts.get(mediaType) || 0) + 1);
      }
      const displayMediaType = (mediaType) => {
        const responseStatus = /^(\d{3}):(.+)$/.exec(mediaType);
        const mimeType = responseStatus ? responseStatus[2] : mediaType;
        const known = new Map([
          ["application/json", "JSON"],
          ["application/xml", "XML"],
          ["text/xml", "XML"],
          ["application/yaml", "YAML"],
          ["text/yaml", "YAML"],
        ]);
        const shortMedia = known.get(mimeType.toLowerCase()) || mimeType;
        return responseStatus ? `${responseStatus[1]} ${shortMedia}` : shortMedia;
      };
      panels.forEach((panel, index) => {
        const mediaType = panel.dataset.mediaType || "Example";
        const label = panel.dataset.exampleLabel || `Example ${index + 1}`;
        const option = document.createElement("option");
        option.value = String(index);
        const visibleMedia = displayMediaType(mediaType);
        const visibleLabel = mediaCounts.get(mediaType) > 1
          ? `${visibleMedia} · ${index + 1}`
          : visibleMedia;
        option.textContent = visibleLabel;
        option.title = `${mediaType} · ${label}`;
        option.setAttribute("aria-label", `${mediaType} · ${label}`);
        select.append(option);
      });
      select.disabled = false;
      select.addEventListener("change", () => {
        activateVariant(Number(select.value));
      });
    } else if (select) {
      select.hidden = true;
      select.disabled = true;
    }

    languageSelect.addEventListener("change", () => {
      showLanguagePanel(Number(languageSelect.value));
    });
    copyButton.addEventListener("click", async () => {
      const activeCode = visibleCodePanels.find((panel) => !panel.hidden)?.querySelector("code");
      if (!activeCode) return;
      try {
        await navigator.clipboard.writeText(activeCode.textContent);
        copyStatus.textContent = "Code copied to clipboard.";
      } catch {
        copyStatus.textContent = "Clipboard access is unavailable; select the code to copy it.";
      }
    });
    activateVariant(0);
  }

  for (const filter of document.querySelectorAll("[data-property-filter]")) {
    const header = filter.closest(".schema-fields-header");
    const siblingFields = header?.nextElementSibling;
    const fields = siblingFields?.classList.contains("fields")
      ? siblingFields
      : filter.closest(".fields");
    const rows = Array.from(fields?.querySelectorAll(":scope > [data-property-row]") || []);
    if (!fields || rows.length === 0) continue;
    filter.disabled = false;
    const empty = fields.querySelector("[data-property-filter-empty]");
    filter.addEventListener("input", () => {
      const query = filter.value.trim().toLowerCase();
      let visible = 0;
      for (const row of rows) {
        const name = (row.dataset.propertyName || "").toLowerCase();
        row.hidden = query.length > 0 && !name.includes(query);
        if (!row.hidden) visible += 1;
      }
      if (empty) empty.hidden = visible !== 0;
    });
  }
  let activeCallbackDialog = null;
  for (const reader of document.querySelectorAll("[data-callback-reader]")) {
    const trigger = reader.querySelector(":scope > [data-callback-trigger]");
    const content = reader.querySelector(":scope > [data-callback-content]");
    if (!(trigger instanceof HTMLElement) || !(content instanceof HTMLElement)) continue;

    const dialog = document.createElement("dialog");
    if (typeof dialog.showModal !== "function") continue;
    dialog.className = "callback-dialog";
    const name = content.dataset.callbackName || trigger.textContent.trim();
    const expression = content.dataset.callbackExpression || "";
    const method = content.dataset.callbackMethod || "";
    const heading = document.createElement("h2");
    heading.className = "callback-dialog-title";
    heading.textContent = name;
    const close = document.createElement("button");
    close.className = "callback-dialog-close";
    close.type = "button";
    close.textContent = "Close";
    close.setAttribute("aria-label", `Close ${name} reader`);
    const header = document.createElement("header");
    header.className = "callback-dialog-header";
    header.append(heading, close);
    const endpoint = document.createElement("div");
    endpoint.className = "callback-dialog-endpoint";
    const methodLabel = document.createElement("span");
    methodLabel.className = `method ${method.toLowerCase() === "post" ? "text-blue-400" : "text-green-400"}`;
    methodLabel.textContent = method;
    const expressionLabel = document.createElement("code");
    expressionLabel.textContent = expression;
    endpoint.append(methodLabel, expressionLabel);
    const body = document.createElement("div");
    body.className = "callback-dialog-body";
    body.tabIndex = -1;
    dialog.append(header, endpoint, body);
    reader.append(dialog);
    trigger.setAttribute("aria-haspopup", "dialog");
    trigger.setAttribute("aria-controls", dialog.id || (dialog.id = `dialog-${reader.id}`));
    trigger.setAttribute("aria-expanded", "false");

    const open = () => {
      if (activeCallbackDialog && activeCallbackDialog !== dialog && activeCallbackDialog.open) {
        activeCallbackDialog.close();
      }
      reader.open = true;
      body.append(content);
      try {
        dialog.showModal();
      } catch {
        reader.insertBefore(content, dialog);
        reader.open = true;
        trigger.setAttribute("aria-expanded", "false");
        return false;
      }
      activeCallbackDialog = dialog;
      trigger.setAttribute("aria-expanded", "true");
      close.focus();
      return true;
    };
    callbackOwners.set(content, { open });
    trigger.addEventListener("click", (event) => {
      event.preventDefault();
      if (!open()) reader.open = true;
    });
    close.addEventListener("click", () => dialog.close());
    dialog.addEventListener("close", () => {
      if (content.parentElement === body) reader.insertBefore(content, dialog);
      reader.open = false;
      trigger.setAttribute("aria-expanded", "false");
      if (activeCallbackDialog === dialog) activeCallbackDialog = null;
      trigger.focus();
    });
  }
  const revealFieldTarget = () => {
    let id = window.location.hash.slice(1);
    try {
      id = decodeURIComponent(id);
    } catch {
      return;
    }
    const target = document.getElementById(id);
    if (!target || !(target.matches("[data-generated-field]") || target.matches(".field h3[id^='field-']"))) return;
    const callbackContent = target.closest("[data-callback-content]");
    if (callbackContent) callbackOwners.get(callbackContent)?.open();
    const legacyTarget = target.matches(".field h3[id^='field-']");
    if (legacyTarget && !target.hasAttribute("tabindex")) target.setAttribute("tabindex", "-1");

    const ancestors = [];
    for (let element = target.parentElement; element; element = element.parentElement) {
      if (element.matches("[data-choice-panel], [data-media-panel]")) ancestors.push(element);
    }
    ancestors.reverse();
    for (const panel of ancestors) {
      panelActivators.get(panel)?.({ resetFilters: false });
    }
    const propertyRows = [];
    for (let element = target; element; element = element.parentElement) {
      if (element.matches("[data-property-row]")) propertyRows.push(element);
    }
    propertyRows.reverse();
    for (const row of propertyRows) {
      const fields = row.parentElement;
      if (!row.hidden || !fields?.classList.contains("fields")) continue;
      const header = fields.previousElementSibling;
      if (!header?.matches(".schema-fields-header")) continue;
      const filter = header.querySelector("[data-property-filter]");
      if (filter?.value) clearPropertyFilter(filter);
    }
    const disclosures = [];
    for (let element = target.parentElement; element; element = element.parentElement) {
      if (element instanceof HTMLDetailsElement) disclosures.push(element);
    }
    disclosures.reverse().forEach((details) => { details.open = true; });
    requestAnimationFrame(() => {
      target.focus({ preventScroll: true });
      target.scrollIntoView({ block: "center" });
    });
  };
  window.addEventListener("hashchange", revealFieldTarget);
  revealFieldTarget();
})();
