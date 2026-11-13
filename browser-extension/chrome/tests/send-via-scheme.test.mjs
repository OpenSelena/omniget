import test from "node:test";
import assert from "node:assert/strict";

import { buildLibregetSchemeUrl, openLibregetScheme } from "../src/send-via-scheme.js";

test("buildLibregetSchemeUrl strips http(s) prefix and prepends libreget://", () => {
  assert.equal(
    buildLibregetSchemeUrl("https://www.youtube.com/watch?v=abc"),
    "libreget://www.youtube.com/watch?v=abc"
  );
  assert.equal(
    buildLibregetSchemeUrl("http://example.com/v"),
    "libreget://example.com/v"
  );
});

test("buildLibregetSchemeUrl preserves an already-libreget URL", () => {
  assert.equal(
    buildLibregetSchemeUrl("libreget://example.com/v"),
    "libreget://example.com/v"
  );
});

test("buildLibregetSchemeUrl wraps magnet/p2p URLs without an extra slash", () => {
  assert.equal(
    buildLibregetSchemeUrl("magnet:?xt=urn:btih:abc"),
    "libreget:magnet:?xt=urn:btih:abc"
  );
  assert.equal(
    buildLibregetSchemeUrl("p2p:foo"),
    "libreget:p2p:foo"
  );
});

test("buildLibregetSchemeUrl returns null for empty / non-string input", () => {
  assert.equal(buildLibregetSchemeUrl(""), null);
  assert.equal(buildLibregetSchemeUrl("   "), null);
  assert.equal(buildLibregetSchemeUrl(null), null);
  assert.equal(buildLibregetSchemeUrl(undefined), null);
  assert.equal(buildLibregetSchemeUrl(42), null);
});

test("openLibregetScheme creates a hidden tab and schedules its removal", async () => {
  const events = [];
  const scheduled = [];

  const tabs = {
    create: async (options) => {
      events.push(["create", options]);
      return { id: 1234 };
    },
    remove: (id) => {
      events.push(["remove", id]);
      return Promise.resolve();
    },
  };

  const schedule = (fn, ms) => {
    scheduled.push({ fn, ms });
    return null;
  };

  const result = await openLibregetScheme("https://example.com/v", {
    tabs,
    schedule,
    tabLifetimeMs: 500,
  });

  assert.deepEqual(result, { ok: true, schemeUrl: "libreget://example.com/v" });
  assert.deepEqual(events, [
    ["create", { url: "libreget://example.com/v", active: false }],
  ]);
  assert.equal(scheduled.length, 1);
  assert.equal(scheduled[0].ms, 500);

  // Run the scheduled cleanup: the throwaway tab should be removed.
  scheduled[0].fn();
  assert.deepEqual(events[1], ["remove", 1234]);
});

test("openLibregetScheme does not navigate the user's active tab", async () => {
  let updateCalled = false;
  const tabs = {
    create: async () => ({ id: 99 }),
    update: async () => {
      updateCalled = true;
    },
    remove: () => Promise.resolve(),
  };

  const result = await openLibregetScheme("https://example.com/v", {
    tabs,
    schedule: () => {},
  });

  assert.equal(result.ok, true);
  assert.equal(updateCalled, false);
});

test("openLibregetScheme returns invalid-url for empty input", async () => {
  const result = await openLibregetScheme("", {
    tabs: { create: async () => ({ id: 1 }), remove: () => Promise.resolve() },
    schedule: () => {},
  });
  assert.deepEqual(result, { ok: false, reason: "invalid-url" });
});

test("openLibregetScheme returns tabs-api-unavailable when tabs.create is missing", async () => {
  const result = await openLibregetScheme("https://example.com/v", {
    tabs: {},
    schedule: () => {},
  });
  assert.deepEqual(result, { ok: false, reason: "tabs-api-unavailable" });
});

test("openLibregetScheme reports tabs-create-failed when tabs.create throws", async () => {
  const tabs = {
    create: async () => {
      throw new Error("permission denied");
    },
    remove: () => Promise.resolve(),
  };

  const result = await openLibregetScheme("https://example.com/v", {
    tabs,
    schedule: () => {},
  });

  assert.deepEqual(result, {
    ok: false,
    reason: "tabs-create-failed",
    message: "permission denied",
  });
});

test("openLibregetScheme tolerates a sync remove() that throws", async () => {
  const tabs = {
    create: async () => ({ id: 7 }),
    remove: () => {
      throw new Error("tab is gone");
    },
  };

  const calls = [];
  const schedule = (fn) => {
    calls.push(fn);
  };

  const result = await openLibregetScheme("https://example.com/v", {
    tabs,
    schedule,
  });

  assert.equal(result.ok, true);
  assert.doesNotThrow(() => calls[0]());
});
