import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AppVisibilityStore } from "./appVisibility.svelte";
import { deferred } from "./testHelpers";

const native = vi.hoisted(() => ({
  listen: vi.fn(),
  isVisible: vi.fn(),
  isMinimized: vi.fn(),
}));

vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => native }));

type VisibilityHandler = (event: { payload: boolean }) => void;
let documentTarget: EventTarget & { hidden: boolean };
let windowTarget: EventTarget & { __TAURI_INTERNALS__?: object };
let onNativeVisibility: VisibilityHandler;
let stopListening: ReturnType<typeof vi.fn<() => void>>;
let cleanups: (() => void)[];

beforeEach(() => {
  documentTarget = Object.assign(new EventTarget(), { hidden: false });
  windowTarget = Object.assign(new EventTarget(), { __TAURI_INTERNALS__: {} });
  vi.stubGlobal("document", documentTarget);
  vi.stubGlobal("window", windowTarget);
  stopListening = vi.fn();
  cleanups = [];
  native.listen.mockReset().mockImplementation((_event, handler) => {
    onNativeVisibility = handler;
    return Promise.resolve(stopListening);
  });
  native.isVisible.mockReset().mockResolvedValue(true);
  native.isMinimized.mockReset().mockResolvedValue(false);
});

afterEach(() => {
  cleanups.forEach((stop) => stop());
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

function startStore() {
  const store = new AppVisibilityStore();
  const stop = store.start();
  cleanups.push(stop);
  return { store, stop };
}

async function initialized() {
  await vi.waitFor(() => expect(native.isMinimized).toHaveBeenCalled());
}

function documentVisibility(hidden: boolean) {
  documentTarget.hidden = hidden;
  documentTarget.dispatchEvent(new Event("visibilitychange"));
}

describe("app visibility", () => {
  it.each([
    [false, false],
    [true, true],
  ])("starts hidden for visible=%s, minimized=%s", async (visible, minimized) => {
    native.isVisible.mockResolvedValue(visible);
    native.isMinimized.mockResolvedValue(minimized);
    const { store } = startStore();
    await initialized();
    expect(store.visible).toBe(false);
  });

  it("updates from tray/minimize/restore events without recurring visibility IPC", async () => {
    const { store } = startStore();
    await initialized();
    expect(native.listen).toHaveBeenCalledWith("app-window-visibility", expect.any(Function));

    onNativeVisibility({ payload: false });
    expect(store.visible).toBe(false);
    onNativeVisibility({ payload: true });
    expect(store.visible).toBe(true);

    vi.useFakeTimers();
    await vi.advanceTimersByTimeAsync(60_000);
    expect(native.isVisible).toHaveBeenCalledTimes(1);
    expect(native.isMinimized).toHaveBeenCalledTimes(1);
  });

  it("keeps a blurred but shown native window visible", async () => {
    const { store } = startStore();
    await initialized();
    windowTarget.dispatchEvent(new Event("blur"));
    windowTarget.dispatchEvent(new Event("focus"));
    onNativeVisibility({ payload: true });
    expect(store.visible).toBe(true);
    expect(native.isVisible).toHaveBeenCalledTimes(1);
  });

  it("requires both WebView and native visibility before resuming", async () => {
    const { store } = startStore();
    await initialized();
    documentVisibility(true);
    onNativeVisibility({ payload: true });
    expect(store.visible).toBe(false);
    documentVisibility(false);
    expect(store.visible).toBe(true);
    onNativeVisibility({ payload: false });
    documentVisibility(false);
    expect(store.visible).toBe(false);
  });

  it("does not let a pending initial read undo markHidden", async () => {
    const read = deferred<boolean>();
    native.isVisible.mockReturnValue(read.promise);
    const { store } = startStore();
    await initialized();
    store.markHidden();
    read.resolve(true);
    await read.promise;
    await Promise.resolve();
    expect(store.visible).toBe(false);
    onNativeVisibility({ payload: true });
    expect(store.visible).toBe(true);
  });

  it("preserves markHidden while the native listener is still registering", async () => {
    const registration = deferred<() => void>();
    native.listen.mockReturnValue(registration.promise);
    const { store } = startStore();
    store.markHidden();
    registration.resolve(stopListening);
    await initialized();
    expect(store.visible).toBe(false);
  });

  it("does not let a pending initial read undo a newer native event", async () => {
    const read = deferred<boolean>();
    native.isVisible.mockReturnValue(read.promise);
    const { store } = startStore();
    await initialized();
    onNativeVisibility({ payload: false });
    read.resolve(true);
    await read.promise;
    await Promise.resolve();
    expect(store.visible).toBe(false);
  });

  it("restores visibility after a rejected hide without undoing later native events", async () => {
    const { store } = startStore();
    await initialized();
    const restore = store.markHidden();
    expect(store.visible).toBe(false);
    restore();
    expect(store.visible).toBe(true);

    const staleRestore = store.markHidden();
    onNativeVisibility({ payload: false });
    staleRestore();
    expect(store.visible).toBe(false);
  });

  it("continues handling native events if the initial visibility read fails", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    native.isVisible.mockRejectedValue(new Error("Window state unavailable"));
    const { store } = startStore();
    await vi.waitFor(() => expect(warn).toHaveBeenCalled());
    onNativeVisibility({ payload: false });
    expect(store.visible).toBe(false);
    onNativeVisibility({ payload: true });
    expect(store.visible).toBe(true);
  });

  it.each(["focus", "visibilitychange"])(
    "recovers a failed initial snapshot on %s without registering twice",
    async (eventName) => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      native.isVisible.mockRejectedValueOnce(new Error("Window state unavailable"));
      const { store } = startStore();
      await vi.waitFor(() => expect(warn).toHaveBeenCalled());
      store.markHidden();

      const target = eventName === "focus" ? windowTarget : documentTarget;
      target.dispatchEvent(new Event(eventName));
      await vi.waitFor(() => expect(store.visible).toBe(true));
      expect(native.listen).toHaveBeenCalledTimes(1);
      expect(native.isVisible).toHaveBeenCalledTimes(2);

      target.dispatchEvent(new Event(eventName));
      await Promise.resolve();
      expect(native.isVisible).toHaveBeenCalledTimes(2);
    },
  );

  it("reads initial native state after registration fails and reconnects on restoration", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    native.listen.mockRejectedValueOnce(new Error("Registration unavailable"));
    native.isVisible.mockResolvedValue(false);
    const { store } = startStore();
    await initialized();
    expect(store.visible).toBe(false);

    store.markHidden();
    native.isVisible.mockResolvedValue(true);
    windowTarget.dispatchEvent(new Event("focus"));
    await vi.waitFor(() => expect(store.visible).toBe(true));
    expect(native.listen).toHaveBeenCalledTimes(2);
    expect(native.isVisible).toHaveBeenCalledTimes(2);
    onNativeVisibility({ payload: false });
    expect(store.visible).toBe(false);
    onNativeVisibility({ payload: true });
    expect(store.visible).toBe(true);
  });

  it("recovers from failed registration and initial reads without a periodic retry", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    native.listen.mockRejectedValueOnce(new Error("Registration unavailable"));
    native.isVisible.mockRejectedValueOnce(new Error("Window state unavailable"));
    const { store } = startStore();
    await vi.waitFor(() => expect(warn).toHaveBeenCalledTimes(2));

    vi.useFakeTimers();
    await vi.advanceTimersByTimeAsync(60_000);
    expect(native.listen).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
    store.markHidden();
    documentVisibility(false);
    await vi.waitFor(() => expect(store.visible).toBe(true));
    expect(native.listen).toHaveBeenCalledTimes(2);
  });

  it("retries a restore event that arrives while failed-listener initialization is pending", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const initialRead = deferred<boolean>();
    native.listen.mockRejectedValueOnce(new Error("Registration unavailable"));
    native.isVisible.mockReturnValueOnce(initialRead.promise);
    const { store } = startStore();
    await initialized();

    store.markHidden();
    documentVisibility(true);
    documentVisibility(false);
    initialRead.resolve(false);
    await vi.waitFor(() => expect(store.visible).toBe(true));
    expect(native.listen).toHaveBeenCalledTimes(2);
    expect(native.isVisible).toHaveBeenCalledTimes(2);
  });

  it("removes a listener even when registration finishes after unmount", async () => {
    const registration = deferred<() => void>();
    native.listen.mockReturnValue(registration.promise);
    const { stop } = startStore();
    await vi.waitFor(() => expect(native.listen).toHaveBeenCalled());
    stop();
    registration.resolve(stopListening);
    await vi.waitFor(() => expect(stopListening).toHaveBeenCalledOnce());
    expect(native.isVisible).not.toHaveBeenCalled();
  });

  it("ignores pending reads and callbacks after cleanup", async () => {
    const read = deferred<boolean>();
    native.isVisible.mockReturnValue(read.promise);
    const { store, stop } = startStore();
    await initialized();
    stop();
    read.resolve(false);
    await read.promise;
    await Promise.resolve();
    onNativeVisibility({ payload: false });
    documentVisibility(true);
    expect(store.visible).toBe(true);
    expect(stopListening).toHaveBeenCalledOnce();
  });

  it("can restart without an old cleanup stopping the new observer", async () => {
    const { store, stop } = startStore();
    await initialized();
    stop();
    cleanups.push(store.start());
    await vi.waitFor(() => expect(native.listen).toHaveBeenCalledTimes(2));
    stop();
    cleanups.push(store.start());
    expect(native.listen).toHaveBeenCalledTimes(2);
    onNativeVisibility({ payload: false });
    expect(store.visible).toBe(false);
  });

  it("uses document visibility in a browser preview without native IPC", async () => {
    delete windowTarget.__TAURI_INTERNALS__;
    const { store } = startStore();
    windowTarget.dispatchEvent(new Event("blur"));
    expect(store.visible).toBe(true);
    documentVisibility(true);
    expect(store.visible).toBe(false);
    documentVisibility(false);
    expect(store.visible).toBe(true);
    await Promise.resolve();
    expect(native.listen).not.toHaveBeenCalled();
  });
});
