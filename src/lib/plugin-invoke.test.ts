import { describe, it, expect, vi, beforeEach } from "vitest";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (cmd: string, args: unknown) => invokeMock(cmd, args),
}));

import { pluginInvoke } from "./plugin-invoke";

describe("pluginInvoke", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("routes through the plugin_command Tauri handler", async () => {
    invokeMock.mockResolvedValueOnce({ ok: true });
    await pluginInvoke("courses", "hotmart_login", { email: "e", password: "p" });
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock.mock.calls[0][0]).toBe("plugin_command");
  });

  it("passes pluginId, command, and args in the payload", async () => {
    invokeMock.mockResolvedValueOnce(undefined);
    await pluginInvoke("telegram", "list_chats", { limit: 20 });
    expect(invokeMock.mock.calls[0][1]).toEqual({
      pluginId: "telegram",
      command: "list_chats",
      args: { limit: 20 },
    });
  });

  it("defaults args to an empty object when not provided", async () => {
    invokeMock.mockResolvedValueOnce("ok");
    await pluginInvoke("convert", "list_formats");
    expect(invokeMock.mock.calls[0][1]).toEqual({
      pluginId: "convert",
      command: "list_formats",
      args: {},
    });
  });

  it("propagates the resolved value", async () => {
    invokeMock.mockResolvedValueOnce({ success: true, count: 3 });
    const result = await pluginInvoke<{ success: boolean; count: number }>(
      "courses",
      "list",
    );
    expect(result).toEqual({ success: true, count: 3 });
  });

  it("propagates rejection from invoke", async () => {
    invokeMock.mockRejectedValueOnce(new Error("plugin not loaded"));
    await expect(pluginInvoke("missing", "cmd")).rejects.toThrow(
      "plugin not loaded",
    );
  });
});
