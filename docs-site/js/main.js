// Copy button, mobile sidebar, active link, "On this page" nav, filter, Ctrl+K
document.addEventListener("DOMContentLoaded", () => {
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

  // Mobile sidebar
  const menuBtn = document.getElementById("menuBtn");
  const sidebar = document.getElementById("sidebar");
  if (menuBtn && sidebar) {
    menuBtn.addEventListener("click", () => sidebar.classList.toggle("open"));
    sidebar.querySelectorAll("a").forEach((a) =>
      a.addEventListener("click", () => sidebar.classList.remove("open"))
    );
  }

  // Active link
  const current = location.pathname.split("/").pop() || "index.html";
  document.querySelectorAll(".sidebar a").forEach((a) => {
    const href = a.getAttribute("href") || "";
    if (href.endsWith(current)) a.classList.add("active");
  });

  // "On this page": built from h2/h3 (with auto ids)
  const tocNav = document.getElementById("tocNav");
  if (tocNav) {
    const slug = (s) =>
      s.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "")
        .replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");
    document.querySelectorAll(".prose h2, .prose h3").forEach((h) => {
      if (!h.id) h.id = slug(h.textContent);
      const a = document.createElement("a");
      a.href = "#" + h.id;
      a.textContent = h.textContent;
      if (h.tagName === "H3") a.classList.add("lvl3");
      tocNav.appendChild(a);
    });
    if (!tocNav.children.length) {
      const aside = document.getElementById("toc");
      if (aside) aside.style.display = "none";
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
