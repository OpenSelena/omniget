import test from "node:test";
import assert from "node:assert/strict";

import { formatCookieSummary, summarizeCookies } from "../src/cookie-summary.js";

test("summarizeCookies returns null for empty or non-array input", () => {
  assert.equal(summarizeCookies([]), null);
  assert.equal(summarizeCookies(null), null);
  assert.equal(summarizeCookies(undefined), null);
  assert.equal(summarizeCookies("cookies"), null);
});

test("summarizeCookies counts cookies and strips leading dot from domain", () => {
  const summary = summarizeCookies([
    { domain: ".example.com", name: "a" },
    { domain: ".example.com", name: "b" },
  ]);
  assert.deepEqual(summary, {
    count: 2,
    primaryDomain: "example.com",
    domainCount: 1,
  });
});

test("summarizeCookies lowercases mixed-case domains", () => {
  const summary = summarizeCookies([
    { domain: ".Example.COM", name: "a" },
    { domain: "example.com", name: "b" },
  ]);
  assert.equal(summary.primaryDomain, "example.com");
  assert.equal(summary.domainCount, 1);
});

test("summarizeCookies picks the domain with the most cookies as primary", () => {
  const summary = summarizeCookies([
    { domain: ".cdn.example.com", name: "a" },
    { domain: ".example.com", name: "b" },
    { domain: ".example.com", name: "c" },
    { domain: ".example.com", name: "d" },
  ]);
  assert.equal(summary.primaryDomain, "example.com");
  assert.equal(summary.domainCount, 2);
  assert.equal(summary.count, 4);
});

test("summarizeCookies breaks ties deterministically by domain", () => {
  const summary = summarizeCookies([
    { domain: "b.example.com", name: "a" },
    { domain: "a.example.com", name: "b" },
  ]);
  assert.equal(summary.primaryDomain, "a.example.com");
});

test("summarizeCookies ignores entries without a domain", () => {
  const summary = summarizeCookies([
    { name: "a" },
    { domain: "", name: "b" },
    { domain: "example.com", name: "c" },
  ]);
  assert.equal(summary.count, 3);
  assert.equal(summary.primaryDomain, "example.com");
  assert.equal(summary.domainCount, 1);
});

test("summarizeCookies returns null when no domain can be extracted", () => {
  assert.equal(summarizeCookies([{ name: "a" }, { domain: "" }]), null);
});

test("formatCookieSummary shows single-domain message", () => {
  assert.equal(
    formatCookieSummary({ count: 12, primaryDomain: "instagram.com", domainCount: 1 }),
    "12 cookies from instagram.com sent",
  );
});

test("formatCookieSummary shows multi-domain message with correct pluralization", () => {
  assert.equal(
    formatCookieSummary({ count: 20, primaryDomain: "instagram.com", domainCount: 2 }),
    "20 cookies from instagram.com and 1 other domain sent",
  );
  assert.equal(
    formatCookieSummary({ count: 30, primaryDomain: "instagram.com", domainCount: 4 }),
    "30 cookies from instagram.com and 3 other domains sent",
  );
});

test("formatCookieSummary returns empty for null/zero/invalid input", () => {
  assert.equal(formatCookieSummary(null), "");
  assert.equal(formatCookieSummary(undefined), "");
  assert.equal(formatCookieSummary({ count: 0, primaryDomain: "x.com", domainCount: 1 }), "");
});

test("summarizeCookies does not leak cookie values or names", () => {
  const summary = summarizeCookies([
    { domain: ".example.com", name: "SESSION_SECRET", value: "s3cret-payload" },
  ]);
  assert.ok(!("name" in summary));
  assert.ok(!("value" in summary));
  assert.ok(!JSON.stringify(summary).includes("SESSION_SECRET"));
  assert.ok(!JSON.stringify(summary).includes("s3cret-payload"));
});
