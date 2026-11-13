const MENU_ID = "libreget-download";

export function registerContextMenu() {
  const api =
    globalThis.browser?.menus ||
    globalThis.browser?.contextMenus ||
    globalThis.chrome?.contextMenus;

  if (!api) return;

  let created = false;
  const doCreate = () => {
    if (created) return;
    created = true;
    try {
      api.create(
        {
          id: MENU_ID,
          title: "Download with LibreGet",
          contexts: ["page", "link", "video", "audio", "image", "selection"],
        },
        () => {
          void globalThis.chrome?.runtime?.lastError;
        }
      );
    } catch {
      // In case api.create throws if already created or on unsupported engine.
    }
  };

  try {
    if (typeof api.removeAll === "function") {
      const res = api.removeAll(() => {
        void globalThis.chrome?.runtime?.lastError;
        doCreate();
      });
      if (res && typeof res.then === "function") {
        res.then(doCreate).catch(doCreate);
      }
    } else {
      doCreate();
    }
  } catch {
    doCreate();
  }
}

export function getContextMenuId() {
  return MENU_ID;
}
