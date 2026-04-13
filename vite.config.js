import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { execSync } from "child_process";
import { readFileSync } from "fs";

const host = process.env.TAURI_DEV_HOST;

function getGitInfo() {
  try {
    return {
      hash: execSync("git rev-parse HEAD").toString().trim(),
      branch: execSync("git rev-parse --abbrev-ref HEAD").toString().trim(),
    };
  } catch {
    return { hash: "unknown", branch: "unknown" };
  }
}

function getVersion() {
  try {
    return JSON.parse(readFileSync("package.json", "utf8")).version;
  } catch {
    return "0.0.0";
  }
}

// https://vite.dev/config/
export default defineConfig(async () => {
  const gitInfo = getGitInfo();

  return {
    plugins: [sveltekit()],

    define: {
      __COMMIT_HASH__: JSON.stringify(gitInfo.hash),
      __GIT_BRANCH__: JSON.stringify(gitInfo.branch),
      __APP_VERSION__: JSON.stringify(getVersion()),
      __BUILD_DATE__: JSON.stringify(new Date().toISOString().split("T")[0]),
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      hmr: host
        ? {
            protocol: "ws",
            host,
            port: 1421,
          }
        : undefined,
      watch: {
        // 3. tell Vite to ignore watching `src-tauri`
        ignored: ["**/src-tauri/**"],
      },
    },
  };
});
