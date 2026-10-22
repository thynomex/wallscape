export class AppVisibilityStore {
  visible = $state(true);
  private started = false;
  private nativeVisible = true;
  private revision = 0;

  start() {
    if (this.started || typeof window === "undefined") {
      return () => {};
    }

    this.started = true;
    const inTauri = "__TAURI_INTERNALS__" in window;
    let stopped = false;
    let unlisten: (() => void) | undefined;
    let nativeStateKnown = false;
    let connecting = false;
    let recoveryRequested = false;

    const update = () => {
      if (!stopped) {
        this.visible = !document.hidden && (!inTauri || this.nativeVisible);
      }
    };

    const connect = async () => {
      if (stopped || !inTauri || (unlisten && nativeStateKnown)) return;
      if (connecting) {
        recoveryRequested = true;
        return;
      }
      connecting = true;
      const readRevision = this.revision;
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        if (stopped) return;
        const windowHandle = getCurrentWindow();
        if (!unlisten) {
          try {
            const stopListening = await windowHandle.listen<boolean>(
              "app-window-visibility",
              ({ payload }) => {
                if (stopped) return;
                this.revision += 1;
                this.nativeVisible = payload;
                nativeStateKnown = true;
                update();
              },
            );

            // Unmount can finish before the asynchronous listener registration.
            if (stopped) {
              stopListening();
              return;
            }
            unlisten = stopListening;
          } catch (error) {
            if (!stopped) {
              console.warn("Failed to listen for window visibility:", error);
            }
          }
        }
        if (stopped) return;

        // Read even if registration failed. A later focus/visibility event
        // retries the listener and refreshes native state without polling.
        const [windowVisible, minimized] = await Promise.all([
          windowHandle.isVisible(),
          windowHandle.isMinimized(),
        ]);
        if (!stopped && this.revision === readRevision) {
          this.nativeVisible = windowVisible && !minimized;
          nativeStateKnown = true;
          update();
        }
      } catch (error) {
        if (!stopped) {
          console.warn("Failed to read window visibility:", error);
        }
      } finally {
        connecting = false;
        if (recoveryRequested) {
          recoveryRequested = false;
          void connect();
        }
      }
    };

    const recover = () => {
      void connect();
    };
    const handleVisibilityChange = () => {
      update();
      recover();
    };

    // WebView visibility is independent of focus: a blurred window may still
    // be visible. Native events supply the hidden/minimized part of the state.
    document.addEventListener("visibilitychange", handleVisibilityChange);
    window.addEventListener("focus", recover);
    update();
    void connect();

    return () => {
      if (stopped) return;
      stopped = true;
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      window.removeEventListener("focus", recover);
      unlisten?.();
      this.started = false;
    };
  }

  markHidden() {
    // Invalidate startup reads that may still report the pre-minimize state.
    const revision = ++this.revision;
    const previousNativeVisible = this.nativeVisible;
    this.nativeVisible = false;
    this.visible = false;

    // A rejected hide/minimize request must not leave the UI paused forever.
    return () => {
      if (this.revision !== revision) return;
      this.nativeVisible = previousNativeVisible;
      this.visible = previousNativeVisible && !document.hidden;
    };
  }
}

export const appVisibility = new AppVisibilityStore();
