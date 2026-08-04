const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (id) => document.getElementById(id);

// ---- tabs ----
function showTab(name) {
  document.querySelectorAll("nav button").forEach((b) => {
    b.classList.toggle("active", b.dataset.tab === name);
  });
  document.querySelectorAll("main section").forEach((s) => {
    s.classList.toggle("active", s.id === name);
  });
  if (name === "history") loadHistory();
}
document.querySelectorAll("nav button").forEach((b) => {
  b.addEventListener("click", () => showTab(b.dataset.tab));
});
listen("goto-tab", (e) => {
  if (e.payload === "setup") $("tab-setup").style.display = "";
  showTab(e.payload);
});

// ---- status line ----
listen("status", (e) => {
  $("statusline").textContent = e.payload;
  const m = $("setup-model");
  if (m) m.textContent = e.payload;
});
listen("model-ready", () => {
  const m = $("setup-model");
  if (m) m.textContent = "Done. The model lives in %LOCALAPPDATA%\\yapping\\models.";
});

// ---- settings ----
let settings = null;
let saveTimer = null;

function scheduleSave() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => invoke("set_settings", { settings }), 350);
}

function bindToggle(id, key) {
  $(id).addEventListener("change", (e) => {
    settings[key] = e.target.checked;
    scheduleSave();
  });
}
function bindText(id, key) {
  $(id).addEventListener("input", (e) => {
    settings[key] = e.target.value;
    scheduleSave();
  });
}

function renderCleanupRows() {
  $("ollama-rows").style.display = settings.cleanup === "ollama" ? "" : "none";
  $("custom-rows").style.display = settings.cleanup === "custom" ? "" : "none";
}

async function loadSettings() {
  settings = await invoke("get_settings");
  $("s-trailing").checked = settings.trailing_space;
  $("s-copyonly").checked = settings.copy_only;
  $("s-overlay").checked = settings.overlay;
  $("s-cleanup").value = settings.cleanup;
  $("s-ollama-url").value = settings.ollama_url;
  $("s-ollama-model").value = settings.ollama_model;
  $("s-custom-url").value = settings.custom_url;
  $("s-custom-key").value = settings.custom_key;
  $("s-custom-model").value = settings.custom_model;
  $("s-prompt").value = settings.prompt;
  renderCleanupRows();
  renderStyles();
}
bindToggle("s-trailing", "trailing_space");
bindToggle("s-copyonly", "copy_only");
bindToggle("s-overlay", "overlay");
bindText("s-ollama-url", "ollama_url");
bindText("s-ollama-model", "ollama_model");
bindText("s-custom-url", "custom_url");
bindText("s-custom-key", "custom_key");
bindText("s-custom-model", "custom_model");
bindText("s-prompt", "prompt");
$("s-cleanup").addEventListener("change", (e) => {
  settings.cleanup = e.target.value;
  renderCleanupRows();
  scheduleSave();
});
$("s-prompt-reset").addEventListener("click", async () => {
  settings.prompt = await invoke("default_prompt");
  $("s-prompt").value = settings.prompt;
  scheduleSave();
});

// ---- per-app styles ----
function escAttr(s) {
  return esc(s).replace(/"/g, "&quot;");
}
function renderStyles() {
  const list = $("styles-list");
  list.innerHTML = settings.styles
    .map((st, i) => {
      const prompt = st.verbatim
        ? ""
        : '<div class="row" style="display:block"><label>Style prompt</label>' +
          '<textarea data-k="prompt" spellcheck="false" style="margin-top:8px">' +
          esc(st.prompt) + "</textarea></div>";
      return (
        '<div class="style-entry" data-i="' + i + '">' +
        '<div class="row"><label>Name</label><input type="text" data-k="name" value="' +
        escAttr(st.name) + '"></div>' +
        '<div class="row"><label>Applies to<span class="sub">Process name contains, comma separated: slack, code.exe, outlook…</span></label>' +
        '<input type="text" data-k="app_patterns" spellcheck="false" value="' +
        escAttr(st.app_patterns.join(", ")) + '"></div>' +
        '<div class="row"><label>Verbatim<span class="sub">Skip cleanup entirely in these apps.</span></label>' +
        '<span class="switch"><input type="checkbox" data-k="verbatim"' +
        (st.verbatim ? " checked" : "") + '><span class="track"></span></span></div>' +
        prompt +
        '<div class="row"><button class="btn ghost small" data-del="' + i + '">Delete style</button></div>' +
        "</div>"
      );
    })
    .join("");
  list.querySelectorAll(".style-entry").forEach((el) => {
    const style = settings.styles[Number(el.dataset.i)];
    el.querySelectorAll("[data-k]").forEach((input) => {
      const key = input.dataset.k;
      if (key === "verbatim") {
        input.addEventListener("change", (e) => {
          style.verbatim = e.target.checked;
          scheduleSave();
          renderStyles();
        });
      } else if (key === "app_patterns") {
        input.addEventListener("input", (e) => {
          style.app_patterns = e.target.value
            .split(",")
            .map((p) => p.trim())
            .filter(Boolean);
          scheduleSave();
        });
      } else {
        input.addEventListener("input", (e) => {
          style[key] = e.target.value;
          scheduleSave();
        });
      }
    });
  });
  list.querySelectorAll("[data-del]").forEach((b) => {
    b.addEventListener("click", () => {
      settings.styles.splice(Number(b.dataset.del), 1);
      scheduleSave();
      renderStyles();
    });
  });
}
$("style-add").addEventListener("click", () => {
  settings.styles.push({ name: "New style", app_patterns: [], prompt: "", verbatim: false });
  scheduleSave();
  renderStyles();
});

// ---- setup ----
$("setup-done").addEventListener("click", () => {
  settings.onboarded = true;
  invoke("set_settings", { settings });
  $("tab-setup").style.display = "none";
  showTab("settings");
});

// ---- history ----
function esc(s) {
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}
async function loadHistory() {
  const entries = await invoke("get_history");
  const list = $("history-list");
  if (!entries.length) {
    list.innerHTML = '<p class="hint">Nothing yet. Hold Ctrl+Win and say something.</p>';
    return;
  }
  list.innerHTML = entries
    .map((e, i) => {
      const main = e.cleaned || e.raw;
      const raw = e.cleaned ? '<div class="raw mono">raw: ' + esc(e.raw) + "</div>" : "";
      return (
        '<div class="hentry"><div class="when mono">' + esc(e.at) + "</div>" +
        '<div class="text">' + esc(main) + "</div>" + raw +
        '<div class="tools"><button class="btn ghost small" data-copy="' + i + '">Copy</button></div></div>'
      );
    })
    .join("");
  list.querySelectorAll("[data-copy]").forEach((b) => {
    b.addEventListener("click", () => {
      const e = entries[Number(b.dataset.copy)];
      navigator.clipboard.writeText(e.cleaned || e.raw);
      b.textContent = "Copied";
      setTimeout(() => (b.textContent = "Copy"), 1200);
    });
  });
}
$("history-clear").addEventListener("click", async () => {
  await invoke("clear_history");
  loadHistory();
});
listen("history-changed", () => {
  if (document.querySelector("#history.active")) loadHistory();
});

// ---- listen mode ----
let listening = false;
function setListening(on) {
  listening = on;
  $("listen-toggle").textContent = on ? "Stop listening" : "Start listening";
  $("listen-toggle").classList.toggle("accent", !on);
  $("listen-toggle").classList.toggle("ghost", on);
}
$("listen-toggle").addEventListener("click", async () => {
  if (listening) {
    invoke("listen_stop");
  } else {
    $("listen-out").textContent = "";
    try {
      await invoke("listen_start");
    } catch (e) {
      $("listen-out").textContent = String(e);
    }
  }
});
listen("listen-state", (e) => setListening(!!e.payload));
listen("listen-text", (e) => {
  const out = $("listen-out");
  out.textContent += (out.textContent ? " " : "") + e.payload;
  out.scrollTop = out.scrollHeight;
});
listen("listen-error", (e) => {
  $("listen-out").textContent = "Error: " + e.payload;
  setListening(false);
});
$("listen-copy").addEventListener("click", () => {
  navigator.clipboard.writeText($("listen-out").textContent);
});

// ---- files ----
$("file-pick").addEventListener("click", () => invoke("pick_and_transcribe"));
listen("file-started", () => {
  $("file-out").textContent = "";
  $("file-progress").style.width = "0%";
  $("file-progress-wrap").style.display = "";
  $("file-pick").disabled = true;
});
listen("file-progress", (e) => {
  $("file-progress").style.width = e.payload + "%";
});
listen("file-done", (e) => {
  $("file-out").textContent = e.payload;
  $("file-progress-wrap").style.display = "none";
  $("file-pick").disabled = false;
});
listen("file-error", (e) => {
  $("file-out").textContent = "Error: " + e.payload;
  $("file-progress-wrap").style.display = "none";
  $("file-pick").disabled = false;
});
$("file-copy").addEventListener("click", () => {
  navigator.clipboard.writeText($("file-out").textContent);
});

// ---- updates ----
$("updates-check").addEventListener("click", async () => {
  const list = $("updates-list");
  list.innerHTML = '<p class="hint">Checking…</p>';
  try {
    const releases = await invoke("check_updates");
    if (!releases.length) {
      list.innerHTML = '<p class="hint ok">You are on the latest version.</p>';
      return;
    }
    list.innerHTML = releases
      .map(
        (r, i) =>
          '<div class="release"><h4>' + esc(r.name) + '</h4><div class="when mono">' +
          esc(r.published) + '</div><div class="notes">' + esc(r.notes) +
          '</div><button class="btn small" data-rel="' + i + '">Download</button></div>'
      )
      .join("");
    list.querySelectorAll("[data-rel]").forEach((b) => {
      b.addEventListener("click", () =>
        invoke("open_url", { url: releases[Number(b.dataset.rel)].url })
      );
    });
  } catch (e) {
    list.innerHTML = '<p class="hint err">' + esc(String(e)) + "</p>";
  }
});

// ---- boot ----
(async () => {
  $("version").textContent = "v" + (await invoke("app_version"));
  await loadSettings();
  if (!settings.onboarded) {
    $("tab-setup").style.display = "";
    showTab("setup");
  }
  if (await invoke("model_ready")) {
    $("statusline").textContent = "Ready. Hold Ctrl+Win to talk";
    const m = $("setup-model");
    if (m) m.textContent = "Done. The model lives in %LOCALAPPDATA%\\yapping\\models.";
  }
})();
