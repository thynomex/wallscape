import { ask } from "@tauri-apps/plugin-dialog";

export interface ConfirmActionOptions {
  title?: string;
  okLabel?: string;
  kind?: "warning" | "info";
}

const inTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * Asks the user to confirm an action with a native dialog. Falls back to
 * `window.confirm` outside the Tauri runtime (browser preview).
 */
export async function confirmAction(
  message: string,
  options: ConfirmActionOptions = {},
): Promise<boolean> {
  if (!inTauri()) return window.confirm(message);

  return ask(message, {
    title: options.title,
    kind: options.kind ?? "warning",
    okLabel: options.okLabel,
    cancelLabel: "Cancel",
  });
}
