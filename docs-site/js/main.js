// Copy button, theme, skip link, scrim, mobile sidebar, active links,
// "On this page" nav with scroll-spy, filter, Ctrl+K
document.addEventListener("DOMContentLoaded", () => {
  // Theme: stored choice wins, otherwise the OS preference. The button is
  // injected so no page template has to change.
  const root = document.documentElement;
  const stored = (() => {
    try { return localStorage.getItem("od-theme"); } catch { return null; }
  })();
  const applyTheme = (t) => {
    root.setAttribute("data-theme", t);
    const btn = document.getElementById("themeBtn");
    if (btn) {
      const dark = t === "dark";
      btn.textContent = dark ? "☀" : "☾";
      btn.setAttribute("aria-label", dark ? "Light theme" : "Dark theme");
      btn.title = dark ? "Light theme" : "Dark theme";
    }
  };
  applyTheme(
    stored === "dark" || stored === "light"
      ? stored
      : (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
  );
  const topInner = document.querySelector(".topbar-inner");
  const searchEl = document.getElementById("searchBtn");
  if (topInner) {
    const themeBtn = document.createElement("button");
    themeBtn.className = "theme-btn";
    themeBtn.id = "themeBtn";
    themeBtn.type = "button";
    themeBtn.addEventListener("click", () => {
      const next = root.getAttribute("data-theme") === "dark" ? "light" : "dark";
      try { localStorage.setItem("od-theme", next); } catch { /* private mode */ }
      applyTheme(next);
    });
    topInner.insertBefore(themeBtn, searchEl || null);
    applyTheme(root.getAttribute("data-theme") || "light");
  }

  // Skip link for keyboard users, injected for the same reason.
  const skip = document.createElement("a");
  skip.className = "skip-link";
  skip.href = "#main";
  skip.textContent = "Skip to content";
  document.body.prepend(skip);
  const layout = document.querySelector("main.layout");
  if (layout && !layout.id) layout.id = "main";
  // Code block copy
  document.querySelectorAll("pre").forEach((pre) => {
    const btn = document.createElement("button");
    btn.className = "copy";
    btn.textContent = "copy";
    btn.addEventListener("click", async () => {
      const text = pre.innerText.replace(/copy$/, "").trim();
      try {
        await navigator.clipboard.writeText(text);
        btn.textContent = "copied";
        setTimeout(() => (btn.textContent = "copy"), 1400);
      } catch {
        btn.textContent = "select + Ctrl+C";
      }
    });
    pre.appendChild(btn);
  });

  // Mobile sidebar + scrim veil, Escape to close
  const menuBtn = document.getElementById("menuBtn");
  const sidebar = document.getElementById("sidebar");
  const scrim = document.createElement("div");
  scrim.className = "scrim";
  scrim.setAttribute("aria-hidden", "true");
  document.body.appendChild(scrim);
  const setSidebar = (open) => {
    if (!sidebar) return;
    sidebar.classList.toggle("open", open);
    document.body.classList.toggle("sidebar-open", open);
    if (menuBtn) menuBtn.setAttribute("aria-expanded", String(open));
  };
  if (menuBtn && sidebar) {
    menuBtn.setAttribute("aria-expanded", "false");
    menuBtn.setAttribute("aria-controls", "sidebar");
    menuBtn.addEventListener("click", () => setSidebar(!sidebar.classList.contains("open")));
    sidebar.querySelectorAll("a").forEach((a) =>
      a.addEventListener("click", () => setSidebar(false))
    );
  }
  scrim.addEventListener("click", () => setSidebar(false));
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      setSidebar(false);
      if (document.activeElement && document.activeElement.blur) document.activeElement.blur();
    }
  });

  // Active link
  const current = location.pathname.split("/").pop() || "index.html";
  document.querySelectorAll(".sidebar a").forEach((a) => {
    const href = a.getAttribute("href") || "";
    if (href.endsWith(current)) a.classList.add("active");
  });

  // "On this page": built from h2/h3 (with auto ids) + scroll-spy
  const tocNav = document.getElementById("tocNav");
  if (tocNav) {
    const slug = (s) =>
      s.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "")
        .replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");
    const links = [];
    document.querySelectorAll(".prose h2, .prose h3").forEach((h) => {
      if (!h.id) h.id = slug(h.textContent);
      const a = document.createElement("a");
      a.href = "#" + h.id;
      a.textContent = h.textContent;
      if (h.tagName === "H3") a.classList.add("lvl3");
      tocNav.appendChild(a);
      links.push([h, a]);
    });
    if (!links.length) {
      const aside = document.getElementById("toc");
      if (aside) aside.style.display = "none";
    } else if ("IntersectionObserver" in window) {
      const byId = new Map(links.map(([h, a]) => [h.id, a]));
      const spy = new IntersectionObserver(
        (entries) => {
          entries.forEach((en) => {
            if (en.isIntersecting) {
              tocNav.querySelectorAll("a.active").forEach((x) => x.classList.remove("active"));
              const a = byId.get(en.target.id);
              if (a) a.classList.add("active");
            }
          });
        },
        { rootMargin: "-80px 0px -70% 0px" }
      );
      links.forEach(([h]) => spy.observe(h));
    }
  }

  // Sidebar filter
  const filter = document.getElementById("sideFilter");
  if (filter && sidebar) {
    filter.addEventListener("input", () => {
      const q = filter.value.trim().toLowerCase();
      sidebar.querySelectorAll("a").forEach((a) => {
        a.classList.toggle("hidden", q !== "" && !a.textContent.toLowerCase().includes(q));
      });
    });
  }

  // Top-bar Search button -> sidebar filter
  const searchBtn = document.getElementById("searchBtn");
  if (searchBtn) {
    searchBtn.addEventListener("click", () => {
      if (sidebar && window.innerWidth <= 800) sidebar.classList.add("open");
      if (filter) filter.focus();
    });
  }

  // Ctrl+K -> filter
  document.addEventListener("keydown", (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      if (filter) {
        if (sidebar) sidebar.classList.add("open");
        filter.focus();
      }
    }
  });
});
