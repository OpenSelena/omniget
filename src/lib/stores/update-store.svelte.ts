import { checkForUpdate, type UpdateInfo } from "$lib/updater";

let updateInfo: UpdateInfo = $state({ available: false });
let isChecking: boolean = $state(false);

export function getUpdateInfo(): UpdateInfo {
  return updateInfo;
}

export function isCheckingForUpdate(): boolean {
  return isChecking;
}

export async function refreshUpdateInfo(): Promise<UpdateInfo> {
  if (isChecking) return updateInfo;
  isChecking = true;
  try {
    updateInfo = await checkForUpdate();
  } finally {
    isChecking = false;
  }
  return updateInfo;
}
