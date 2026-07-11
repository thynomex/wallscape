export interface ImportDropState {
  active: boolean;
  itemCount: number;
}

export interface ImportDropListenerOptions {
  onStateChange: (state: ImportDropState) => void;
  onDrop: (paths: string[]) => void | Promise<void>;
}

export async function registerImportDropListener(
  options: ImportDropListenerOptions,
) {
  const { getCurrentWebview } = await import("@tauri-apps/api/webview");

  return getCurrentWebview().onDragDropEvent((event) => {
    if (event.payload.type === "enter" || event.payload.type === "over") {
      if ("paths" in event.payload) {
        options.onStateChange({
          active: event.payload.paths.length > 0,
          itemCount: event.payload.paths.length,
        });
      }
      return;
    }

    if (event.payload.type === "leave") {
      options.onStateChange({ active: false, itemCount: 0 });
      return;
    }

    if (event.payload.type === "drop") {
      options.onStateChange({ active: false, itemCount: 0 });
      if (event.payload.paths.length) {
        void options.onDrop(event.payload.paths);
      }
    }
  });
}
