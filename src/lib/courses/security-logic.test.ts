import { describe, it, expect } from "vitest";
import {
  sanitizeCookieString,
  shouldFallbackToWebview,
  formatTokenIdentity,
  resolveClientCredentials,
  sanitizePortalName,
  parseUdemyResults,
} from "./security-logic";

describe("Course Security & Platform Logic", () => {
  describe("Cookie Sanitization (Issue 7)", () => {
    it("should strip CRLF (carriage return and line feed) to prevent header injection", () => {
      const rawCookie = "sessionid=abc123\r\nX-Injected: evil\nSet-Cookie: pwn=1";
      const cleaned = sanitizeCookieString(rawCookie);
      expect(cleaned).not.toContain("\r");
      expect(cleaned).not.toContain("\n");
      expect(cleaned).toBe("sessionid=abc123X-Injected: evilSet-Cookie: pwn=1");
    });

    it("should strip null bytes and ASCII control characters", () => {
      const rawCookie = "token=secret\x00\x07\x1b";
      const cleaned = sanitizeCookieString(rawCookie);
      expect(cleaned).toBe("token=secret");
    });
  });

  describe("Udemy Fallback Decision (Issue 5)", () => {
    it("should NOT fallback to webview when API succeeds with 0 courses", () => {
      const apiResult = { success: true, courses: [] };
      expect(shouldFallbackToWebview(apiResult)).toBe(false);
    });

    it("should NOT fallback to webview when API succeeds with courses", () => {
      const apiResult = { success: true, courses: [{ id: 1, title: "Course 1" }] };
      expect(shouldFallbackToWebview(apiResult)).toBe(false);
    });

    it("should fallback to webview when API returns an error", () => {
      const apiResult = { success: false, error: "401 Unauthorized / Token Expired" };
      expect(shouldFallbackToWebview(apiResult)).toBe(true);
    });
  });

  describe("Token User Identity Resolution (Issue 10)", () => {
    it("should extract user email if provided in token payload", () => {
      expect(formatTokenIdentity("student@example.com:secret-key")).toBe("student@example.com:secret-key");
    });

    it("should format a recognizable truncated token identity for anonymous tokens", () => {
      expect(formatTokenIdentity("eyJhGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9")).toBe("Token User (eyJhGc)");
    });

    it("should gracefully handle short tokens", () => {
      expect(formatTokenIdentity("abc")).toBe("Token User");
    });
  });

  describe("Client Credential Resolution (Issue 6)", () => {
    it("should provide valid client credentials with environment override support", () => {
      const creds = resolveClientCredentials();
      expect(creds.clientId).toBeTruthy();
      expect(creds.clientSecret).toBeTruthy();
    });
  });

  describe("Udemy Portal Sanitization & Error Handling (Issue 72)", () => {
    it("should sanitize tracking and non-portal subdomains to www", () => {
      expect(sanitizePortalName("")).toBe("www");
      expect(sanitizePortalName("www")).toBe("www");
      expect(sanitizePortalName("trk")).toBe("www");
      expect(sanitizePortalName(".trk")).toBe("www");
      expect(sanitizePortalName("support")).toBe("www");
      expect(sanitizePortalName("blog")).toBe("www");
      expect(sanitizePortalName("img-c")).toBe("www");
      expect(sanitizePortalName("s")).toBe("www");
    });

    it("should preserve enterprise subdomains", () => {
      expect(sanitizePortalName("acmewidgets")).toBe("acmewidgets");
      expect(sanitizePortalName(".enterprise-corp")).toBe("enterprise-corp");
    });

    it("should successfully extract results array when present", () => {
      const payload = { results: [{ id: 101, title: "Rust Mastery" }] };
      const parsed = parseUdemyResults(payload);
      expect(parsed).toHaveLength(1);
      expect(parsed[0]).toEqual({ id: 101, title: "Rust Mastery" });
    });

    it("should throw error with details when authorization fails", () => {
      const unauthorizedPayload = { detail: "You do not have permission to perform this action." };
      expect(() => parseUdemyResults(unauthorizedPayload)).toThrow(
        "Udemy authorization error: You do not have permission to perform this action."
      );
    });

    it("should throw error when results key is missing from payload", () => {
      const invalidPayload = { unexpected: true };
      expect(() => parseUdemyResults(invalidPayload)).toThrow(
        "Invalid API response: missing 'results' array"
      );
    });
  });
});
