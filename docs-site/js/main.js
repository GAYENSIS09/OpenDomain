// Copy button, theme, skip link, scrim, mobile sidebar, active links,
// "On this page" nav with scroll-spy, filter, Ctrl+K
document.addEventListener("DOMContentLoaded", () => {
  // Theme: stored choice wins, otherwise the OS preference. The button is
  // injected so no page template has to change.
  const root = document.documentElement;
  const stored = (() => {
    try { return localStorage.getItem("opendomain-theme"); } catch { return null; }
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
      try { localStorage.setItem("opendomain-theme", next); } catch { /* private mode */ }
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
  // Code blocks — each `pre` is wrapped by main.js into a `.codeblock`
  // with a header (detected language + copy button), so the button never
  // sits over the code, and the text snapshot taken before any DOM change
  // means a block that itself ends in the word "copy" is never truncated.
  const codeLang = (text) => {
    const t = (text || "").replace(/^\s+/, "");
    if (/^\$|^\s*(opendomain|cargo|git|mkdir|cd|npm|pnpm|python|node|pip)\s/.test(t)) return "console";
    if (/^\s*(fn |let |use |struct |impl |#\[|pub )/m.test(t)) return "rust";
    // TOML : clé = valeur (chaîne, tableau, table anonyme, booléen, nombre)
    if (/^\s*[A-Za-z_][\w.-]*\s*=\s*(?:"[\s\S]*"|\[|\{|true|false|-?\d)/m.test(t)) return "toml";
    return "text";
  };
  document.querySelectorAll("article pre").forEach((pre) => {
    const code = pre.querySelector("code");
    let text = ((code || pre).textContent || "").replace(/\r?\n+$/, "");
    // Les sources HTML contiennent parfois des fences Markdown qui ont
    // fui (```toml … ```) : on les retire ici et le tag devient le
    // langage affiché — l'en-tête lit TOML et « copy » copie du TOML nu.
    let forcedLang = null;
    const outerFence = text.match(/^\s*```([A-Za-z0-9+#-]*)[ \t]*\r?\n([\s\S]*?)\r?\n?[ \t]*```\s*$/);
    if (outerFence) {
      forcedLang = outerFence[1] || null;
      text = outerFence[2];
    }
    const wrap = document.createElement("div");
    wrap.className = "codeblock";
    const head = document.createElement("div");
    head.className = "codehead";
    const langEl = document.createElement("span");
    langEl.className = "clang";
    langEl.textContent = forcedLang || codeLang(text);
    const btn = document.createElement("button");
    btn.className = "copy";
    btn.type = "button";
    btn.textContent = "copy";
    btn.setAttribute("aria-label", "Copy code to clipboard");
    btn.addEventListener("click", async () => {
      const done = (msg) => {
        btn.textContent = msg;
        setTimeout(() => (btn.textContent = "copy"), 1500);
      };
      try {
        await navigator.clipboard.writeText(text);
        done("copied ✓");
      } catch {
        // Clipboard API is unavailable on file:// or plain http contexts —
        // fall back to the textarea trick instead of just saying no.
        try {
          const ta = document.createElement("textarea");
          ta.value = text;
          ta.setAttribute("readonly", "");
          ta.style.position = "fixed";
          ta.style.opacity = "0";
          document.body.appendChild(ta);
          ta.select();
          document.execCommand("copy");
          ta.remove();
          done("copied ✓");
        } catch {
          done("Ctrl+C");
        }
      }
    });
    head.append(langEl, btn);
    pre.replaceWith(wrap);
    wrap.append(head, pre);
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

  // Pagination : cartes « précédent / suivant » en fin d'article.
  // L'ordre des pages vient de la barre latérale ; une page peut livrer
  // sa propre `.page-nav` (concepts.html) : rien n'est alors généré. Un
  // ancien `.next-links` commençant par « Next: » est consommé : son
  // lien devient la carte Suivant et le texte qui suit (le slogan)
  // devient la description de cette carte. Les descriptions des autres
  // cartes reprennent le sous-titre de chaque page.
  try {
    if (!document.querySelector(".page-nav")) {
      const DESC = {
        "Introduction": "Understand the domain before coding, and prove every rule.",
        "Getting started": "Install the tool, create a project, get your first verdict.",
        "Concepts": "The three ideas everything else follows from.",
        "Artifacts": "Text for humans, evidence for the machine.",
        "The 9 gates": "Nine questions you no longer get to forget.",
        "CLI reference": "Four intents: locate, link, ask, evolve.",
        "With opencode": "The same assistant, with a work contract.",
        "The harness": "One door, gates everywhere, evidence archived.",
        "Theory": "The complete framework specification.",
      };
      const pages = Array.from(document.querySelectorAll(".sidebar a")).map((a) => ({
        label: a.textContent.trim(),
        href: a.getAttribute("href") || "",
      }));
      const cur = location.pathname.split("/").pop() || "index.html";
      const idx = pages.findIndex((p) => p.href.split("/").pop() === cur);
      const article = document.querySelector("article.prose");
      if (idx !== -1 && article) {
        let next = null;
        const legacy = document.querySelector(".next-links");
        if (legacy && /^\s*Next\s*:/i.test(legacy.textContent)) {
          const a = legacy.querySelector("a");
          if (a) {
            const tagline = (legacy.textContent.split(a.textContent).slice(1).join("") || "")
              .replace(/^[\s—–-]+/, "").trim();
            next = {
              label: a.textContent.trim(),
              href: a.getAttribute("href") || "",
              desc: tagline || (DESC[a.textContent.trim()] || ""),
            };
            legacy.remove();
          }
        }
        if (!next && idx + 1 < pages.length) {
          const p = pages[idx + 1];
          next = { label: p.label, href: p.href, desc: DESC[p.label] || "" };
        }
        const prev = idx > 0 ? pages[idx - 1] : null;
        if (prev || next) {
          const nav = document.createElement("nav");
          nav.className = "page-nav";
          nav.setAttribute("aria-label", "Page navigation");
          const card = (cls, dir, label, href, desc) => {
            const a = document.createElement("a");
            a.className = "page-card " + cls;
            a.href = href;
            const dirEl = document.createElement("span");
            dirEl.className = "dir";
            dirEl.textContent = dir;
            const titleEl = document.createElement("span");
            titleEl.className = "pt";
            titleEl.textContent = label;
            a.append(dirEl, titleEl);
            if (desc) {
              const d = document.createElement("span");
              d.className = "pd";
              d.textContent = desc;
              a.append(d);
            }
            nav.append(a);
          };
          if (prev) card("prev", "← Previous", prev.label, prev.href, DESC[prev.label] || "");
          if (next) card("next", "Next →", next.label, next.href, next.desc);
          article.appendChild(nav);
        }
      }
    }
  } catch (err) {
    /* La pagination est décorative : ne jamais casser la page pour elle. */
  }
});
