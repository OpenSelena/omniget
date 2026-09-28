import { checkPythonDependencies, ensureBinaries, type DependencyStatus } from '$lib/api/open_nami';

let status = $state<DependencyStatus | null>(null);
let checking = $state(false);
let inflight: Promise<void> | null = null;

export function getDependencyStatus(): DependencyStatus | null {
  return status;
}

export function isCheckingDependencies(): boolean {
  return checking;
}

export async function ensureDependenciesChecked(force = false): Promise<DependencyStatus | null> {
  if (status && !force) return status;
  if (inflight) {
    await inflight;
    return status;
  }

  checking = true;
  inflight = (async () => {
    try {
      status = await checkPythonDependencies();
    } catch (err: unknown) {
      status = {
        ok: false,
        message: err instanceof Error ? err.message : String(err),
        gallery_dl_version: null,
        yt_dlp_version: null,
        auto_provision_available: true
      };
    } finally {
      checking = false;
      inflight = null;
    }
  })();

  await inflight;
  return status;
}

export async function autoInstallEngines(): Promise<DependencyStatus | null> {
  checking = true;
  try {
    status = await ensureBinaries();
  } catch (err: unknown) {
    status = {
      ok: false,
      message: err instanceof Error ? err.message : String(err),
      gallery_dl_version: null,
      yt_dlp_version: null,
      auto_provision_available: true
    };
  } finally {
    checking = false;
  }
  return status;
}

