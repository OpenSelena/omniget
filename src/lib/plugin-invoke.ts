import { invoke } from "@tauri-apps/api/core";

export async function pluginInvoke<T>(
  pluginId: string,
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  return invoke<T>("plugin_command", {
    pluginId,
    command,
    args,
  });
}
