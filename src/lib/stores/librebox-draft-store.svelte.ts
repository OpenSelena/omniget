let draftUrl = $state("");

export function getLibreboxDraftUrl(): string {
  return draftUrl;
}

export function setLibreboxDraftUrl(url: string) {
  draftUrl = url;
}

export function clearLibreboxDraftUrl() {
  draftUrl = "";
}

