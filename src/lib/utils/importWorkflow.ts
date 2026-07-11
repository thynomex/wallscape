import type {
  BatchImportItem,
  BatchImportResult,
  ImportScanResult,
  StoreActionResult,
  Wallpaper,
} from "$lib/types/wallpaper";
import { prepareImportItems } from "$lib/utils/importPreparation";
import { formatBatchImportMessage } from "$lib/utils/importSummary";
import { importFailureMessage, type ImportSource } from "$lib/utils/toastMessages";

interface ImportWorkflowActions {
  scanImportPaths(paths: string[]): Promise<ImportScanResult>;
  importWallpapers(items: BatchImportItem[]): Promise<StoreActionResult<BatchImportResult>>;
}

interface ImportWorkflowState {
  setStatus(status: string | null): void;
  setWarnings(warnings: string[]): void;
}

export type ImportWorkflowErrorKind = "selection" | "import" | "unexpected";

export type ImportWorkflowOutcome =
  | {
      ok: true;
      message: string;
      thumbnailWallpaper: Wallpaper | null;
    }
  | {
      ok: false;
      error: string;
      kind: ImportWorkflowErrorKind;
    };

export async function importSelectedWallpaperPaths(
  paths: string[],
  source: ImportSource,
  actions: ImportWorkflowActions,
  state: ImportWorkflowState,
): Promise<ImportWorkflowOutcome> {
  try {
    state.setStatus("Scanning import selection...");
    state.setWarnings([]);
    const scan = await actions.scanImportPaths(paths);

    if (!scan.files.length) {
      const message =
        scan.rejected[0]?.reason ?? "No supported video wallpapers were found.";
      state.setStatus(null);
      state.setWarnings(rejectedPathWarnings(scan));
      return { ok: false, error: message, kind: "selection" };
    }

    const items = await prepareImportItems(scan.files, {
      onStatus: (status) => state.setStatus(status),
    });

    state.setWarnings(rejectedPathWarnings(scan));
    const result = await actions.importWallpapers(items);
    if (!result.ok) {
      return { ok: false, error: result.error, kind: "import" };
    }

    const { imported, duplicates } = result.value;
    return {
      ok: true,
      message: formatBatchImportMessage(result.value),
      thumbnailWallpaper: imported[0] ?? duplicates[0] ?? null,
    };
  } catch (error) {
    return {
      ok: false,
      error: importFailureMessage(source, error),
      kind: "unexpected",
    };
  }
}

function rejectedPathWarnings(scan: ImportScanResult): string[] {
  return scan.rejected.map((item) => `${item.path}: ${item.reason}`);
}
