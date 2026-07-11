class AppVisibilityStore {
  visible = $state(true);
  private started = false;

  start() {
    if (this.started || typeof window === "undefined") {
      return () => {};
    }

    this.started = true;
    const inTauri = "__TAURI_INTERNALS__" in window;
    let stopped = false;

    const update = async () => {
      let visible = !document.hidden;

      if (inTauri) {
        try {
          const { getCurrentWindow } = await import("@tauri-apps/api/window");
          const windowHandle = getCurrentWindow();
          const [windowVisible, minimized] = await Promise.all([
            windowHandle.isVisible(),
            windowHandle.isMinimized(),
          ]);
          visible = visible && windowVisible && !minimized;
        } catch (error) {
          console.warn("Failed to read window visibility:", error);
        }
      }

      if (!stopped) {
        this.visible = visible;
      }
    };

    const handleVisibilityChange = () => {
      void update();
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    window.addEventListener("focus", handleVisibilityChange);
    window.addEventListener("blur", handleVisibilityChange);
    const interval = window.setInterval(handleVisibilityChange, 1000);
    void update();

    return () => {
      stopped = true;
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      window.removeEventListener("focus", handleVisibilityChange);
      window.removeEventListener("blur", handleVisibilityChange);
      window.clearInterval(interval);
      this.started = false;
    };
  }

  markHidden() {
    this.visible = false;
  }
}

export const appVisibility = new AppVisibilityStore();
