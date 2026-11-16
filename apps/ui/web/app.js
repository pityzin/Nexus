import { compareBenchmarkReports, formatAddress, formatBytes, parseLosslessJson, validateBenchmarkReport } from "/data.js";

const byId = (id) => document.getElementById(id);
const views = ["overview", "binary", "nir", "performance", "plugins"];
const state = { status: null, plugins: [], current: null, rawNir: null, pluginResult: null, sessions: [], reports: [], sectionPage: 0, sectionFilter: "", upload: null };
const PAGE_SIZE = 50;

function text(element, value) {
  if (element) element.textContent = value == null ? "—" : String(value);
}

function setError(message) {
  const alert = byId("global-error");
  text(alert, message);
  alert.hidden = !message;
}

function activateView(name) {
  if (!views.includes(name)) return;
  for (const view of views) {
    const section = byId(`view-${view}`);
    section.hidden = view !== name;
    section.classList.toggle("active-view", view === name);
  }
  document.querySelectorAll("[data-view-link]").forEach((link) => {
    const active = link.dataset.viewLink === name;
    link.classList.toggle("active", active);
    if (active) link.setAttribute("aria-current", "page");
    else link.removeAttribute("aria-current");
  });
  const labels = { overview: "Overview", binary: "Binary explorer", nir: "NIR metadata", performance: "Performance", plugins: "Plugin exports" };
  text(byId("current-view-label"), labels[name]);
  setError("");
}

function el(tag, className, value) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (value !== undefined) node.textContent = String(value);
  return node;
}

async function connect() {
  const dot = byId("connection-dot");
  try {
    const response = await fetch("/api/v1/status", { credentials: "omit", cache: "no-store" });
    if (!response.ok) throw new Error(`engine status returned HTTP ${response.status}`);
    const status = await response.json();
    if (status.protocol_version !== 1 || !Array.isArray(status.supported_formats)) throw new Error("unsupported local UI protocol");
    state.status = status;
    const pluginResponse = await fetch("/api/v1/plugins", { credentials: "omit", cache: "no-store" });
    if (!pluginResponse.ok) throw new Error(`plugin registry returned HTTP ${pluginResponse.status}`);
    const pluginRegistry = await pluginResponse.json();
    if (pluginRegistry.plugin_api_version !== status.plugin_api_version || !Array.isArray(pluginRegistry.plugins)) throw new Error("invalid plugin registry response");
    state.plugins = pluginRegistry.plugins;
    dot.className = "dot";
    text(byId("connection-label"), "Local engine connected");
    text(byId("engine-version"), status.engine_version);
    text(byId("footer-engine"), `Engine ${status.engine_version} · protocol ${status.protocol_version}`);
    renderCapabilities(status);
    renderPlugins(pluginRegistry);
  } catch (error) {
    dot.className = "dot offline";
    text(byId("connection-label"), "Local engine unavailable");
    text(byId("footer-engine"), "Could not connect to the NEXUS local engine");
    setError(`The local NEXUS service could not be reached. Start it with “nexus ui” and reload this page. ${error.message}`);
  }
}

function renderCapabilities(status) {
  const implemented = new Set(status.implemented);
  const unavailable = new Set(status.unavailable);
  const features = [
    ["binary_metadata", "PE / ELF metadata", "Header and section parsing"],
    ["headers", "Format headers", "PE and ELF header fields"],
    ["sections", "Section explorer", "Paged table with filtering"],
    ["nir_metadata", "NIR metadata v1", "Format-neutral metadata output"],
    ["parse_wall_time", "Parser timing", "Current import parse duration"],
    ["benchmark_report_import", "Benchmark reports", "Import saved Phase 06 JSON"],
    ["plugin_discovery", "Plugin discovery", "Build-registered Rust extensions"],
    ["metadata_plugin_export", "Metadata plugin export", "Run metadata-summary on parsed PE/ELF input"],
    ["disassembly", "Disassembly", "Instruction engine not implemented"],
    ["cfg", "Control-flow graph", "CFG analysis not implemented"],
    ["symbols", "Symbols and xrefs", "Symbol analysis not implemented"],
  ];
  const list = byId("capability-list");
  list.replaceChildren();
  for (const [key, name, detail] of features) {
    const row = el("div", "capability");
    const marker = el("span", `dot ${implemented.has(key) ? "" : "unavailable"}`.trim());
    marker.setAttribute("aria-hidden", "true");
    const copy = el("span");
    copy.append(el("span", "capability-name", name), el("span", "capability-detail", implemented.has(key) ? detail : (unavailable.has(key) ? detail : "Not reported by this engine")));
    row.append(marker, copy);
    list.append(row);
  }
  byId("open-file-main").disabled = !implemented.has("binary_metadata");
}

function renderPlugins(registry) {
  const list = byId("plugin-list");
  list.replaceChildren();
  text(byId("plugin-count"), registry.plugins.length);
  text(byId("plugin-api-version"), `PLUGIN API v${registry.plugin_api_version}`);
  if (registry.plugins.length === 0) {
    list.append(el("div", "empty-inline", "No plugins are registered in this build."));
    return;
  }
  for (const plugin of registry.plugins) {
    const card = el("div", "plugin-card");
    const title = el("div", "plugin-card-title");
    title.append(el("strong", "plugin-name", plugin.id), el("span", `plugin-state ${plugin.state === "enabled" ? "enabled" : "disabled"}`, plugin.state));
    const details = el("div", "plugin-card-details", `Version ${plugin.version} · ${plugin.extension_point} · host API ${plugin.min_host_api}–${plugin.max_host_api}`);
    const capabilities = el("div", "plugin-card-details", `Capabilities: ${plugin.capabilities.join(", ") || "none"}`);
    card.append(title, details, capabilities);
    list.append(card);
  }
  const enabled = registry.plugins.some((plugin) => plugin.id === "metadata-summary" && plugin.state === "enabled");
  byId("run-metadata-plugin").disabled = !enabled || !state.current;
}

function chooseBinary() {
  byId("binary-file").click();
}

function beginUpload(file) {
  if (!state.status) return setError("The NEXUS engine has not connected yet.");
  if (!file || file.size === 0) return setError("Choose a non-empty file.");
  if (file.size > state.status.max_upload_bytes) return setError(`File exceeds the local UI limit of ${formatBytes(state.status.max_upload_bytes)}.`);
  setError("");
  const startedAt = performance.now();
  const xhr = new XMLHttpRequest();
  state.upload = xhr;
  const status = byId("upload-status");
  status.hidden = false;
  text(byId("upload-label"), `Reading ${file.name}`);
  text(byId("upload-detail"), `0% · ${formatBytes(file.size)}`);
  byId("upload-progress").value = 0;
  xhr.open("POST", "/api/v1/inspect");
  xhr.withCredentials = false;
  xhr.setRequestHeader("Content-Type", "application/octet-stream");
  xhr.upload.onprogress = (event) => {
    if (!event.lengthComputable) return;
    const percent = Math.min(100, (event.loaded / event.total) * 100);
    byId("upload-progress").value = percent;
    text(byId("upload-detail"), `${percent.toFixed(0)}% · ${formatBytes(event.loaded)} of ${formatBytes(event.total)}`);
  };
  xhr.onerror = () => finishUploadError("Could not send the selected bytes to the local NEXUS service.");
  xhr.onabort = () => {
    status.hidden = true;
    state.upload = null;
    text(byId("upload-detail"), "Upload cancelled");
  };
  xhr.onload = () => {
    state.upload = null;
    status.hidden = true;
    let envelope;
    try {
      envelope = JSON.parse(xhr.responseText);
    } catch {
      return setError("The local engine returned an invalid response.");
    }
    if (xhr.status !== 200) return setError(envelope?.error?.message || `Inspection failed with HTTP ${xhr.status}.`);
    try {
      const metadata = parseLosslessJson(envelope.metadata_json);
      if (metadata.schema_version !== state.status.nir_schema_version || !Array.isArray(metadata.sections)) throw new Error("invalid NIR metadata response");
      state.current = { fileName: file.name, file, metadata, parseNs: envelope.parse_elapsed_ns, transferMs: performance.now() - startedAt, fileBytes: file.size };
      state.rawNir = envelope.metadata_json;
      state.pluginResult = null;
      byId("plugin-result").textContent = "";
      byId("plugin-result").hidden = true;
      byId("download-plugin-result").disabled = true;
      state.sessions.unshift({ fileName: file.name, format: metadata.format, bytes: file.size, at: new Date().toLocaleTimeString() });
      state.sessions = state.sessions.slice(0, 5);
      renderCurrent();
      activateView("binary");
      text(byId("upload-label"), `Inspected ${file.name}`);
      text(byId("upload-detail"), `${formatBytes(file.size)} · binary remained data`);
    } catch (error) {
      setError(`Could not read NIR metadata: ${error.message}`);
    }
  };
  xhr.send(file);
}

function finishUploadError(message) {
  state.upload = null;
  byId("upload-status").hidden = true;
  setError(message);
}

function renderCurrent() {
  const current = state.current;
  if (!current) return;
  const metadata = current.metadata;
  text(byId("welcome-title"), current.fileName);
  text(byId("welcome-copy"), `${metadata.format.toUpperCase()} · ${metadata.architecture} · ${formatBytes(current.fileBytes)}. Parsed by the local NEXUS engine; the file was never executed.`);
  text(byId("overview-file"), current.fileName);
  text(byId("overview-size"), formatBytes(metadata.file_size));
  text(byId("overview-format"), metadata.format.toUpperCase());
  text(byId("overview-architecture"), `${metadata.bitness}-bit · ${metadata.endianness}`);
  text(byId("overview-sections"), metadata.sections.length);
  text(byId("overview-time"), formatNs(current.parseNs));
  byId("binary-empty").hidden = true;
  byId("binary-content").hidden = false;
  byId("export-metadata").disabled = false;
  byId("run-metadata-plugin").disabled = !state.plugins.some((plugin) => plugin.id === "metadata-summary" && plugin.state === "enabled");
  byId("plugin-run-empty").hidden = true;
  byId("nir-empty").hidden = true;
  byId("nir-content").hidden = false;
  text(byId("nir-json"), state.rawNir);
  renderMetadataCards(current);
  renderHeaders(metadata.headers);
  state.sectionPage = 0;
  renderSections();
  renderLiveProfile(current);
  renderSessions();
}

function renderMetadataCards(current) {
  const metadata = current.metadata;
  const entry = metadata.entry_point_kind === "rva" ? `RVA ${formatAddress(metadata.entry_point)}` : formatAddress(metadata.entry_point);
  const cards = [
    ["FILE", current.fileName, formatBytes(metadata.file_size)],
    ["FORMAT", metadata.format.toUpperCase(), `${metadata.bitness}-bit · ${metadata.endianness}`],
    ["ARCHITECTURE", metadata.architecture, "From the parsed machine field"],
    ["ENTRY POINT", entry, metadata.entry_point_kind.toUpperCase()],
  ];
  const container = byId("metadata-cards");
  container.replaceChildren();
  for (const [label, value, foot] of cards) {
    const card = el("article", "meta-card");
    card.append(el("div", "metric-label", label), el("div", "metric-value", value), el("div", "metric-foot", foot));
    container.append(card);
  }
}

function renderHeaders(headers) {
  const container = byId("header-fields");
  container.replaceChildren();
  for (const [key, value] of Object.entries(headers)) {
    const field = el("dl", "header-field");
    field.append(el("dt", "", key.replaceAll("_", " ")));
    let display;
    if (Array.isArray(value)) display = `${value.length} entries · ${JSON.stringify(value)}`;
    else if (typeof value === "string" && /^\d+$/.test(value) && /offset|address|base|entry/i.test(key)) display = `${value} (${formatAddress(value)})`;
    else display = typeof value === "object" && value !== null ? JSON.stringify(value) : String(value);
    field.append(el("dd", "", display));
    container.append(field);
  }
}

function sectionMatches(section, query) {
  if (!query) return true;
  const address = `${formatAddress(section.virtual_address)} ${formatAddress(section.raw_data_offset)}`;
  const permission = `${section.permissions.read ? "r" : ""}${section.permissions.write ? "w" : ""}${section.permissions.execute ? "x" : ""}`;
  return `${section.name} ${section.index} ${address} ${permission} ${section.virtual_size} ${section.raw_data_size}`.toLowerCase().includes(query);
}

function renderSections() {
  if (!state.current) return;
  const sections = state.current.metadata.sections;
  const filtered = sections.filter((section) => sectionMatches(section, state.sectionFilter));
  const pages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  state.sectionPage = Math.min(state.sectionPage, pages - 1);
  const start = state.sectionPage * PAGE_SIZE;
  const page = filtered.slice(start, start + PAGE_SIZE);
  const body = byId("sections-body");
  body.replaceChildren();
  for (const section of page) {
    const row = document.createElement("tr");
    row.append(el("td", "mono", section.index), el("td", "section-name", section.name || "(unnamed)"));
    row.append(el("td", "mono", formatAddress(section.virtual_address)));
    row.append(el("td", "mono", formatBytes(section.virtual_size)));
    row.append(el("td", "mono", formatAddress(section.raw_data_offset)));
    row.append(el("td", "mono", formatBytes(section.raw_data_size)));
    const permissions = el("td", "");
    const badges = el("span", "perm");
    for (const [key, glyph] of [["read", "R"], ["write", "W"], ["execute", "X"]]) {
      badges.append(el("span", section.permissions[key] ? "enabled" : "", glyph));
    }
    permissions.append(badges);
    row.append(permissions);
    body.append(row);
  }
  text(byId("section-count"), filtered.length);
  text(byId("section-page-label"), filtered.length ? `${start + 1}–${Math.min(start + page.length, filtered.length)} of ${filtered.length} sections` : "No matching sections");
  byId("section-prev").disabled = state.sectionPage === 0;
  byId("section-next").disabled = state.sectionPage >= pages - 1;
}

function renderLiveProfile(current) {
  const list = byId("profile-live");
  byId("profile-empty").hidden = true;
  list.hidden = false;
  const stages = [
    ["Metadata parse", current.parseNs],
    ["File transfer", current.transferMs * 1_000_000],
  ];
  const max = Math.max(...stages.map((stage) => stage[1]), 1);
  list.replaceChildren();
  for (const [name, nanoseconds] of stages) {
    const row = el("div", "profile-stage");
    row.append(el("span", "stage-name", name));
    const track = el("progress", "stage-track");
    track.max = max;
    track.value = nanoseconds;
    track.setAttribute("aria-label", `${name} duration`);
    row.append(track, el("span", "stage-time", formatNs(nanoseconds)));
    list.append(row);
  }
}

function renderSessions() {
  const container = byId("recent-sessions");
  container.replaceChildren();
  if (state.sessions.length === 0) {
    container.append(el("div", "empty-inline", "Imported files stay in this tab only."));
    return;
  }
  for (const session of state.sessions) {
    const row = el("div", "recent-item");
    row.append(el("span", "recent-name", session.fileName));
    row.append(el("span", "recent-detail", `${session.format.toUpperCase()} · ${session.at}`));
    container.append(row);
  }
}

function renderReports() {
  const empty = byId("reports-empty");
  const list = byId("reports-list");
  const body = byId("reports-body");
  empty.hidden = state.reports.length > 0;
  list.replaceChildren();
  body.replaceChildren();
  byId("clear-reports").disabled = state.reports.length === 0;
  for (const item of state.reports) {
    const report = item.report;
    const chip = el("div", "report-chip");
    chip.append(el("strong", "", item.name), el("span", "", `${report.benchmark} · median ${formatNs(report.statistics.median_ns)}`));
    list.append(chip);
    const row = document.createElement("tr");
    row.append(el("td", "", item.name), el("td", "mono", report.benchmark), el("td", "mono", report.statistics.sample_count));
    row.append(el("td", "mono", formatNs(report.statistics.median_ns)), el("td", "mono", formatNs(report.statistics.mean_ns)));
    row.append(el("td", "", `${report.environment.os} ${report.environment.os_version || ""} · ${report.environment.cpu || "CPU unreported"}`));
    body.append(row);
  }
  const comparisons = byId("comparison-results");
  comparisons.replaceChildren();
  for (let index = 1; index < state.reports.length; index += 1) {
    const current = state.reports[index];
    for (let baseIndex = 0; baseIndex < index; baseIndex += 1) {
      const baseline = state.reports[baseIndex];
      const result = compareBenchmarkReports(current.report, baseline.report);
      if (result.status === "incompatible") continue;
      const row = el("div", "comparison-row");
      const change = result.medianChange == null ? "" : ` · median ${result.medianChange >= 0 ? "+" : ""}${result.medianChange.toFixed(1)}%`;
      row.append(el("span", "", `${current.name} vs ${baseline.name}${change}`));
      row.append(el("span", `comparison-status ${result.status}`, result.status.replaceAll("_", " ")));
      comparisons.append(row);
    }
  }
}

async function importReports(files) {
  if (files.length > 50 || state.reports.length + files.length > 100) return setError("Import up to 50 benchmark reports at a time (100 stored per tab).");
  for (const file of files) {
    if (file.size > 4 * 1024 * 1024) {
      setError(`${file.name} exceeds the 4 MiB report import limit.`);
      continue;
    }
    try {
      const report = validateBenchmarkReport(JSON.parse(await file.text()));
      state.reports.push({ name: file.name, report });
    } catch (error) {
      setError(`${file.name}: ${error.message}`);
    }
  }
  renderReports();
}

function exportMetadata() {
  if (!state.rawNir || !state.current) return;
  const blob = new Blob([state.rawNir], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = `${safeFileStem(state.current.fileName)}.nir.json`;
  link.click();
  URL.revokeObjectURL(url);
}

async function runMetadataPlugin() {
  if (!state.current?.file) return setError("Load a PE or ELF binary before running the metadata exporter.");
  const button = byId("run-metadata-plugin");
  button.disabled = true;
  text(button, "Running...");
  try {
    const response = await fetch("/api/v1/plugins/metadata-summary", {
      method: "POST",
      credentials: "omit",
      cache: "no-store",
      headers: { "Content-Type": "application/octet-stream" },
      body: state.current.file,
    });
    const raw = await response.text();
    let result;
    try { result = JSON.parse(raw); } catch { throw new Error("plugin endpoint returned invalid JSON"); }
    if (!response.ok) throw new Error(result?.error?.message || `plugin export failed with HTTP ${response.status}`);
    if (result.schema_version !== 1 || result.plugin !== "metadata-summary") throw new Error("plugin endpoint returned an unsupported report");
    state.pluginResult = raw;
    byId("plugin-result").textContent = raw;
    byId("plugin-result").hidden = false;
    byId("download-plugin-result").disabled = false;
    byId("plugin-run-empty").hidden = true;
    setError("");
  } catch (error) {
    setError(`Metadata plugin failed: ${error.message}`);
  } finally {
    text(button, "Run on loaded binary");
    button.disabled = !state.current || !state.plugins.some((plugin) => plugin.id === "metadata-summary" && plugin.state === "enabled");
  }
}

function downloadPluginResult() {
  if (!state.pluginResult || !state.current) return;
  const blob = new Blob([state.pluginResult], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = `${safeFileStem(state.current.fileName)}.metadata-summary.json`;
  link.click();
  URL.revokeObjectURL(url);
}

function safeFileStem(name) {
  return name.replace(/[\\/:*?"<>|\u0000-\u001f]/g, "_").slice(0, 120) || "binary";
}

function formatNs(value) {
  if (!Number.isFinite(value) || value < 0) return "—";
  if (value < 1000) return `${value.toFixed(0)} ns`;
  if (value < 1_000_000) return `${(value / 1000).toFixed(2)} μs`;
  if (value < 1_000_000_000) return `${(value / 1_000_000).toFixed(2)} ms`;
  return `${(value / 1_000_000_000).toFixed(2)} s`;
}

document.querySelectorAll("[data-view-link]").forEach((link) => link.addEventListener("click", (event) => {
  event.preventDefault();
  activateView(link.dataset.viewLink);
}));
for (const id of ["open-file-top", "open-file-main", "open-file-empty"]) byId(id).addEventListener("click", chooseBinary);
byId("binary-file").addEventListener("change", (event) => {
  const file = event.target.files?.[0];
  if (file) beginUpload(file);
  event.target.value = "";
});
byId("cancel-upload").addEventListener("click", () => state.upload?.abort());
byId("section-search").addEventListener("input", (event) => {
  state.sectionFilter = event.target.value.trim().toLowerCase();
  state.sectionPage = 0;
  renderSections();
});
byId("section-prev").addEventListener("click", () => { state.sectionPage -= 1; renderSections(); });
byId("section-next").addEventListener("click", () => { state.sectionPage += 1; renderSections(); });
byId("export-metadata").addEventListener("click", exportMetadata);
byId("run-metadata-plugin").addEventListener("click", runMetadataPlugin);
byId("download-plugin-result").addEventListener("click", downloadPluginResult);
byId("copy-nir").addEventListener("click", async () => {
  if (!state.rawNir) return;
  try {
    await navigator.clipboard.writeText(state.rawNir);
    text(byId("copy-nir"), "Copied");
    setTimeout(() => text(byId("copy-nir"), "Copy JSON"), 1400);
  } catch {
    setError("Clipboard access is unavailable in this browser context.");
  }
});
byId("report-files").addEventListener("change", async (event) => {
  await importReports(Array.from(event.target.files || []));
  event.target.value = "";
});
byId("clear-reports").addEventListener("click", () => {
  state.reports = [];
  renderReports();
});

renderReports();
connect();
