/* illuminate · v4 — progressive enhancement for the landing + dashboard pages.
 *
 * Loaded with `defer`, so the DOM is parsed before this runs. Everything here
 * is optional polish: a live clock, copy-to-clipboard on `[data-copy]` /
 * `[data-clipboard]` chips with a `.toast` confirmation, and active-section
 * highlighting in the left rail. The pages render fully without it; this only
 * adds behaviour. Authored to be 404-free and side-effect-safe. */
(function () {
  "use strict";

  var doc = document;

  /* ---- live clock (UTC HH:MM:SS · any [data-clock] target) -------------- */
  function startClock() {
    var els = doc.querySelectorAll("[data-clock]");
    if (!els.length) return;
    function tick() {
      var t = new Date().toISOString().slice(11, 19) + " UTC";
      for (var i = 0; i < els.length; i++) els[i].textContent = t;
    }
    tick();
    setInterval(tick, 1000);
  }

  /* ---- copy-to-clipboard + toast ---------------------------------------- */
  var toastTimer = null;
  function showToast(msg) {
    var toast = doc.getElementById("toast");
    if (!toast) return;
    if (msg) toast.textContent = msg;
    toast.classList.add("show");
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(function () {
      toast.classList.remove("show");
    }, 1500);
  }

  function copyText(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      return navigator.clipboard.writeText(text).catch(function () {
        fallbackCopy(text);
      });
    }
    fallbackCopy(text);
    return Promise.resolve();
  }

  function fallbackCopy(text) {
    try {
      var ta = doc.createElement("textarea");
      ta.value = text;
      ta.setAttribute("readonly", "");
      ta.style.position = "absolute";
      ta.style.left = "-9999px";
      doc.body.appendChild(ta);
      ta.select();
      doc.execCommand("copy");
      doc.body.removeChild(ta);
    } catch (e) {
      /* clipboard unavailable — nothing to do */
    }
  }

  function wireCopy() {
    doc.addEventListener("click", function (ev) {
      var el = ev.target;
      while (el && el !== doc.body) {
        if (el.hasAttribute && (el.hasAttribute("data-copy") || el.hasAttribute("data-clipboard"))) {
          var text = el.getAttribute("data-copy") || el.getAttribute("data-clipboard") || "";
          if (text) {
            copyText(text);
            showToast("copied");
          }
          return;
        }
        el = el.parentNode;
      }
    });
  }

  /* ---- live dashboard data (GET /api/dashboard) ------------------------- *
   * The dashboard ships with static mock numbers so it renders fully offline
   * (and so the file-served Playwright smoke test passes). When served by
   * `illuminate wiki serve`, this fetches the live envelope and swaps the
   * mock values in. ANY failure (offline, non-2xx, bad JSON) is swallowed and
   * the static markup is left exactly as authored — graceful degradation. */

  function resolvePath(obj, path) {
    var cur = obj;
    var parts = path.split(".");
    for (var i = 0; i < parts.length; i++) {
      if (cur == null || typeof cur !== "object") return undefined;
      cur = cur[parts[i]];
    }
    return cur;
  }

  function fmtNum(v) {
    if (typeof v === "number" && isFinite(v)) {
      return v.toLocaleString("en-US");
    }
    return v == null ? "" : String(v);
  }

  /* Replace the text node(s) of `el` while preserving trailing element
   * children (e.g. the `<span class="u">nodes</span>` unit suffix). */
  function setLeadingText(el, text) {
    var first = el.firstChild;
    if (first && first.nodeType === 3) {
      first.nodeValue = text;
    } else {
      el.insertBefore(doc.createTextNode(text), el.firstChild);
    }
  }

  /* data-bind-rows="dotted.path" on a <template> → for each array item, clone
   * the template and fill its `[data-bind-cell="field"]` descendants from that
   * item, then replace the sibling `.ar-list` placeholder rows with the clones.
   * A non-array / empty / missing path leaves the authored placeholder markup
   * exactly as written (graceful degradation). The destination list is the
   * template's next element sibling (the authored `<div class="ar-list">`). */
  function applyRowRepeaters(data) {
    var tmpls = doc.querySelectorAll("template[data-bind-rows]");
    for (var i = 0; i < tmpls.length; i++) {
      var tmpl = tmpls[i];
      var rows = resolvePath(data, tmpl.getAttribute("data-bind-rows"));
      if (!Array.isArray(rows) || rows.length === 0) continue;

      var dest = tmpl.nextElementSibling;
      while (dest && dest.nodeType !== 1) dest = dest.nextElementSibling;
      if (!dest || typeof tmpl.content === "undefined") continue;

      var frag = doc.createDocumentFragment();
      for (var r = 0; r < rows.length; r++) {
        var item = rows[r];
        if (item == null || typeof item !== "object") continue;
        var clone = tmpl.content.firstElementChild.cloneNode(true);
        var cells = clone.querySelectorAll("[data-bind-cell]");
        for (var c = 0; c < cells.length; c++) {
          var field = cells[c].getAttribute("data-bind-cell");
          var cv = item[field];
          if (cv !== undefined && cv !== null) cells[c].textContent = fmtNum(cv);
        }
        frag.appendChild(clone);
      }
      if (frag.childNodes.length) {
        dest.textContent = "";
        dest.appendChild(frag);
      }
    }
  }

  /* ---- mermaid living-diagram (lazy CDN load + offline fallback) -------- *
   * The graph panel ships a `#mermaid-graph` container with an authored static
   * `.mermaid-fallback` (a <pre> of the diagram source) that renders fully
   * offline. We try to upgrade it to a real mermaid SVG by lazily loading the
   * mermaid renderer from a CDN; the diagram source is the live `diagram`
   * field of /api/dashboard when present, else the authored fallback text.
   *
   * EVERY failure path (CDN blocked, offline, parse error) is swallowed and the
   * `.mermaid-fallback` is left visible — no console error escapes that the
   * test harness does not already treat as benign. The CDN <script> is injected
   * at most once; a later live `diagram` re-renders into the same container. */
  var MERMAID_CDN =
    "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js";
  var mermaidState = { loading: false, ready: false, failed: false, pending: null };
  var mermaidSeq = 0;

  function mermaidContainer() {
    return doc.getElementById("mermaid-graph");
  }

  /* The diagram source: prefer a live `data-bind-diagram` path, else the
   * authored `.mermaid-src` fallback text. Returns "" when nothing usable. */
  function diagramSource(container, data) {
    if (data) {
      var path = container.getAttribute("data-bind-diagram");
      if (path) {
        var v = resolvePath(data, path);
        if (typeof v === "string" && v.trim()) return v;
      }
    }
    var src = container.querySelector(".mermaid-src");
    return src ? (src.textContent || "").trim() : "";
  }

  function drawMermaid(source) {
    var container = mermaidContainer();
    if (!container || !mermaidState.ready || !window.mermaid || !source) return;
    var id = "mermaid-svg-" + ++mermaidSeq;
    try {
      var render = window.mermaid.render;
      if (typeof render !== "function") return;
      var out = render(id, source);
      /* mermaid@10+ render() returns a Promise; older versions a string. */
      if (out && typeof out.then === "function") {
        out
          .then(function (res) {
            var svg = res && res.svg ? res.svg : res;
            paintMermaid(container, svg);
          })
          .catch(function () {
            /* parse/render failure — keep the authored fallback visible */
          });
      } else if (typeof out === "string") {
        paintMermaid(container, out);
      }
    } catch (e) {
      /* renderer threw — keep the authored fallback visible */
    }
  }

  function paintMermaid(container, svg) {
    if (!svg) return;
    var holder = container.querySelector(".mermaid-rendered");
    if (!holder) {
      holder = doc.createElement("div");
      holder.className = "mermaid-rendered";
      container.appendChild(holder);
    }
    holder.innerHTML = svg;
    var fallback = container.querySelector(".mermaid-fallback");
    if (fallback) fallback.setAttribute("hidden", "");
    container.classList.add("is-rendered");
  }

  function loadMermaid(source) {
    var container = mermaidContainer();
    if (!container) return;
    if (mermaidState.failed) return; /* CDN already known-bad — stay on fallback */
    if (mermaidState.ready) {
      drawMermaid(source || diagramSource(container, null));
      return;
    }
    mermaidState.pending = source || diagramSource(container, null);
    if (mermaidState.loading) return;
    mermaidState.loading = true;

    var s = doc.createElement("script");
    s.src = MERMAID_CDN;
    s.async = true;
    s.onload = function () {
      mermaidState.loading = false;
      if (!window.mermaid || typeof window.mermaid.initialize !== "function") {
        mermaidState.failed = true;
        return;
      }
      try {
        window.mermaid.initialize({
          startOnLoad: false,
          theme: "dark",
          securityLevel: "strict",
          themeVariables: {
            fontFamily: "'Geist Mono', ui-monospace, monospace",
            primaryColor: "#16151c",
            primaryBorderColor: "#ffb547",
            primaryTextColor: "#ece9f2",
            lineColor: "#6ed1c8",
          },
        });
        mermaidState.ready = true;
        drawMermaid(mermaidState.pending);
      } catch (e) {
        mermaidState.failed = true;
      }
    };
    s.onerror = function () {
      /* CDN blocked / offline — leave the authored fallback visible, no throw */
      mermaidState.loading = false;
      mermaidState.failed = true;
    };
    doc.head.appendChild(s);
  }

  function initMermaid() {
    var container = mermaidContainer();
    if (!container) return;
    loadMermaid(diagramSource(container, null));
  }

  function applyBindings(data) {
    /* Reflect the real project name in the tab title (the <title> element has
     * no data-bind hook of its own). Falls back to the authored "…" only if the
     * envelope omits `project`. */
    if (data && typeof data.project === "string" && data.project) {
      doc.title = "illuminate · dashboard — " + data.project;
    }
    /* If the live envelope carries a `diagram` field, (re)render it; otherwise
     * the offline fallback / static-source render already kicked off at init.
     * When no live `diagram` is supplied but a real `project` is, substitute it
     * into the authored fallback diagram source (`repo["…"]`) so the rendered
     * mermaid label reflects the actual repo rather than the demo literal. */
    var mc = mermaidContainer();
    if (mc) {
      var live = diagramSource(mc, data);
      if (live) {
        loadMermaid(live);
      } else if (data && typeof data.project === "string" && data.project) {
        var srcEl = mc.querySelector(".mermaid-src");
        if (srcEl) {
          var patched = (srcEl.textContent || "").replace(
            /repo\["[^"]*"\]/,
            'repo["' + data.project + '"]'
          );
          if (patched && patched !== srcEl.textContent) {
            srcEl.textContent = patched;
            loadMermaid(patched);
          }
        }
      }
    }
    /* data-bind="dotted.path" → element text = formatted value */
    var bound = doc.querySelectorAll("[data-bind]");
    for (var i = 0; i < bound.length; i++) {
      var v = resolvePath(data, bound[i].getAttribute("data-bind"));
      if (v !== undefined && v !== null) bound[i].textContent = fmtNum(v);
    }
    /* data-bind-prepend="dotted.path" → set leading text, keep child units */
    var pre = doc.querySelectorAll("[data-bind-prepend]");
    for (var j = 0; j < pre.length; j++) {
      var pv = resolvePath(data, pre[j].getAttribute("data-bind-prepend"));
      if (pv !== undefined && pv !== null) setLeadingText(pre[j], fmtNum(pv));
    }
    /* data-bind-tmpl="...{dotted.path}..." → interpolate paths into a string */
    var tmpls = doc.querySelectorAll("[data-bind-tmpl]");
    for (var k = 0; k < tmpls.length; k++) {
      var tmpl = tmpls[k].getAttribute("data-bind-tmpl");
      var ok = true;
      var out = tmpl.replace(/\{([^}]+)\}/g, function (_m, p) {
        var val = resolvePath(data, p.trim());
        if (val === undefined || val === null) {
          ok = false;
          return "";
        }
        return fmtNum(val);
      });
      if (ok) tmpls[k].textContent = out;
    }
    /* data-bind-rows="path" on a <template> → list repeater (see above). */
    applyRowRepeaters(data);
  }

  function hydrateDashboard() {
    /* No bind hooks on this page (e.g. the landing page) → nothing to do. */
    if (
      !doc.querySelector("[data-bind]") &&
      !doc.querySelector("[data-bind-prepend]") &&
      !doc.querySelector("[data-bind-tmpl]") &&
      !doc.querySelector("[data-bind-rows]") &&
      !doc.querySelector("[data-bind-diagram]")
    ) {
      return;
    }
    if (!window.fetch) return;
    fetch("/api/dashboard", { headers: { accept: "application/json" } })
      .then(function (r) {
        if (!r.ok) throw new Error("status " + r.status);
        return r.json();
      })
      .then(function (data) {
        if (data && typeof data === "object") applyBindings(data);
      })
      .catch(function () {
        /* offline / not served by illuminate — keep the static mock markup */
      });
  }

  /* ---- active section highlighting in the left rail --------------------- */
  function wireScrollSpy() {
    var links = doc.querySelectorAll(".vnav a[href^='#']");
    if (!links.length || !("IntersectionObserver" in window)) return;

    var byId = {};
    var targets = [];
    for (var i = 0; i < links.length; i++) {
      var id = links[i].getAttribute("href").slice(1);
      var sec = id && doc.getElementById(id);
      if (sec) {
        byId[id] = links[i];
        targets.push(sec);
      }
    }
    if (!targets.length) return;

    var obs = new IntersectionObserver(
      function (entries) {
        for (var j = 0; j < entries.length; j++) {
          var e = entries[j];
          if (e.isIntersecting && byId[e.target.id]) {
            for (var k = 0; k < links.length; k++) links[k].classList.remove("active");
            byId[e.target.id].classList.add("active");
          }
        }
      },
      { rootMargin: "-40% 0px -55% 0px", threshold: 0 }
    );
    for (var m = 0; m < targets.length; m++) obs.observe(targets[m]);
  }

  function init() {
    startClock();
    wireCopy();
    wireScrollSpy();
    initMermaid();
    hydrateDashboard();
  }

  if (doc.readyState === "loading") {
    doc.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
