import {
  loadBridgeConfig,
  saveBridgeConfig,
  checkBridgeHealth,
  discoverBridgeEndpoint,
  trimEndpoint,
  autoPair,
} from "../src/bridge-client.js";

const endpointInput = document.getElementById("endpoint");
const tokenInput = document.getElementById("token");
const revealBtn = document.getElementById("reveal");
const form = document.getElementById("pair-form");
const testBtn = document.getElementById("test");
const pairNowBtn = document.getElementById("pair-now");
const statusEl = document.getElementById("status");
const welcomeHeader = document.getElementById("welcome-header");
const settingsHeader = document.getElementById("settings-header");
const discoveryEl = document.getElementById("discovery-status");
const discoveryMessage = document.getElementById("discovery-message");
const endpointHint = document.getElementById("endpoint-hint");
const advancedDetails = document.getElementById("advanced");

const FALLBACK_ENDPOINT = "http://127.0.0.1:47720";

function resolvedEndpoint() {
  return trimEndpoint(endpointInput.value) || FALLBACK_ENDPOINT;
}

function setStatus(message, kind) {
  statusEl.textContent = message ?? "";
  statusEl.classList.remove("ok", "error");
  if (kind === "ok") statusEl.classList.add("ok");
  if (kind === "error") statusEl.classList.add("error");
}

function setDiscovery(state, message) {
  discoveryEl.classList.remove("found", "missing");
  if (state === "found") discoveryEl.classList.add("found");
  if (state === "missing") discoveryEl.classList.add("missing");
  discoveryMessage.textContent = message;
}

// --- Auto-pairing -----------------------------------------------------------
// The desktop app opens a single-use ~120s pairing window when the user
// clicks "Pair extension" in Settings. While this page is visible we poll
// `GET /v1/pair` every 5 seconds so pairing completes within seconds of that
// click (the background service worker only retries once per minute).
const AUTOPAIR_POLL_MS = 5000;
let pairPollTimer = null;

async function onPairedSuccess() {
  const { endpoint, token } = await loadBridgeConfig();
  endpointInput.value = endpoint || "";
  tokenInput.value = token || "";
  setDiscovery("found", `Paired with OmniGet at ${endpoint}.`);
  setStatus("Paired automatically — you're all set.", "ok");
  stopPairPolling();
}

async function tryAutoPair() {
  const result = await autoPair().catch(() => ({ ok: false }));
  if (!result?.ok) return false;
  if (result.reason === "already-paired") {
    stopPairPolling();
    return true;
  }
  await onPairedSuccess();
  return true;
}

function startPairPolling() {
  if (pairPollTimer !== null) return;
  pairPollTimer = setInterval(() => {
    void tryAutoPair();
  }, AUTOPAIR_POLL_MS);
}

function stopPairPolling() {
  if (pairPollTimer !== null) {
    clearInterval(pairPollTimer);
    pairPollTimer = null;
  }
}

document.addEventListener("visibilitychange", () => {
  if (document.hidden) {
    stopPairPolling();
    return;
  }
  loadBridgeConfig().then(({ token }) => {
    if (!token) {
      void tryAutoPair();
      startPairPolling();
    }
  });
});

async function init() {
  const { endpoint, token } = await loadBridgeConfig();
  const alreadyPaired = Boolean(token);

  // While unpaired, keep trying to grab the token automatically: once
  // immediately (the user may already have a pairing window open in the
  // app) and then every few seconds while this page stays visible.
  if (!alreadyPaired) {
    void tryAutoPair();
    if (!document.hidden) startPairPolling();
  }

  // Show the welcome heading on first run, the regular settings heading
  // once the user is already paired (they're here to inspect / change).
  if (alreadyPaired) {
    settingsHeader.hidden = false;
  } else {
    welcomeHeader.hidden = false;
  }

  endpointInput.value = endpoint || "";
  tokenInput.value = token || "";

  // Skip auto-discovery if the user is already paired AND we have a stored
  // endpoint that responds — they're probably here to change the token, no
  // need to overwrite the URL they trust.
  if (alreadyPaired) {
    const result = await checkBridgeHealth(endpoint);
    if (result.ok) {
      const versionSuffix = result.version ? ` (v${result.version})` : "";
      setDiscovery("found", `Connected to OmniGet${versionSuffix} at ${endpoint}.`);
      return;
    }
    setDiscovery(
      "missing",
      `Couldn't reach the saved endpoint ${endpoint}. Probing default ports…`
    );
  }

  const found = await discoverBridgeEndpoint();
  if (found) {
    endpointInput.value = found.endpoint;
    const versionSuffix = found.version ? ` (v${found.version})` : "";
    setDiscovery(
      "found",
      `Found OmniGet${versionSuffix} on ${found.endpoint}. Paste the token from OmniGet → Settings → Network → Browser extension to finish.`
    );
    endpointHint.textContent =
      "Auto-detected — change only if your OmniGet runs on a different host.";
    return;
  }

  // Discovery failed: open the Advanced disclosure so the user can supply
  // the endpoint manually.
  if (advancedDetails) advancedDetails.open = true;
  setDiscovery(
    "missing",
    "OmniGet doesn't seem to be running. Launch the desktop app, then refresh this page — or set the endpoint manually below."
  );
}

revealBtn.addEventListener("click", () => {
  const next = tokenInput.type === "password" ? "text" : "password";
  tokenInput.type = next;
  revealBtn.textContent = next === "password" ? "Show" : "Hide";
  revealBtn.setAttribute("aria-pressed", String(next !== "password"));
});

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const endpoint = resolvedEndpoint();
  const token = tokenInput.value.trim();
  if (!token) {
    setStatus("Paste the pairing token first.", "error");
    return;
  }
  await saveBridgeConfig({ endpoint, token });
  setStatus("Saved. The extension will use this token from now on.", "ok");
});

if (pairNowBtn) {
  pairNowBtn.addEventListener("click", async () => {
    setStatus("Trying to pair automatically…");
    const paired = await tryAutoPair();
    if (paired) return;
    setStatus(
      "No open pairing window found. In OmniGet, go to Settings → Network → Browser extension and click \"Pair extension\", then try again (or just wait — this page keeps retrying).",
      "error"
    );
    if (!document.hidden) startPairPolling();
  });
}

testBtn.addEventListener("click", async () => {
  const endpoint = resolvedEndpoint();
  setStatus("Testing connection…");
  const result = await checkBridgeHealth(endpoint);
  if (result.ok) {
    const versionSuffix = result.version ? ` (v${result.version})` : "";
    setStatus(`Connected to OmniGet${versionSuffix} at ${endpoint}.`, "ok");
  } else {
    setStatus(
      `Could not reach OmniGet at ${endpoint}. Make sure the app is running.`,
      "error"
    );
  }
});

init();
