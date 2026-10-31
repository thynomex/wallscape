export type ToastType = "success" | "error" | "info" | "warning";

export interface Toast {
  id: string;
  message: string;
  type: ToastType;
  duration: number;
  thumbnail?: string;
  dismissing?: boolean;
  paused?: boolean;
}

class ToastStore {
  toasts = $state<Toast[]>([]);
  private autoDismissTimers = new Map<string, ReturnType<typeof setTimeout>>();
  private removalTimers = new Map<string, ReturnType<typeof setTimeout>>();
  /** When each running auto-dismiss timer fires, so a pause can keep the rest. */
  private deadlines = new Map<string, number>();
  /** Time left on a paused toast's auto-dismiss timer. */
  private remaining = new Map<string, number>();

  /**
   * Show a toast notification.
   * @param message - The message to display
   * @param type - The type of toast (success, error, info, warning)
   * @param duration - Auto-dismiss duration in milliseconds (default: 3000)
   * @param thumbnail - Optional thumbnail URL for the toast
   */
  show(message: string, type: ToastType = "info", duration = 3000, thumbnail?: string) {
    const id = `toast-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
    const toast: Toast = { id, message, type, duration, thumbnail };

    this.toasts = [...this.toasts, toast];

    if (duration > 0) {
      this.startAutoDismiss(id, duration);
    }
  }

  success(message: string, duration = 3000, thumbnail?: string) {
    this.show(message, "success", duration, thumbnail);
  }

  error(message: string, duration = 6000, thumbnail?: string) {
    this.show(message, "error", duration, thumbnail);
  }

  info(message: string, duration = 3000, thumbnail?: string) {
    this.show(message, "info", duration, thumbnail);
  }

  warning(message: string, duration = 3500, thumbnail?: string) {
    this.show(message, "warning", duration, thumbnail);
  }

  /** Stop the auto-dismiss countdown, e.g. while the toast is hovered or focused. */
  pause(id: string) {
    const deadline = this.deadlines.get(id);
    if (deadline === undefined) return;

    this.clearTimer(this.autoDismissTimers, id);
    this.deadlines.delete(id);
    this.remaining.set(id, Math.max(0, deadline - Date.now()));
    this.setPaused(id, true);
  }

  /** Continue a paused countdown with the time it had left. */
  resume(id: string) {
    const remaining = this.remaining.get(id);
    if (remaining === undefined) return;

    this.remaining.delete(id);
    this.setPaused(id, false);
    this.startAutoDismiss(id, remaining);
  }

  dismiss(id: string) {
    const toast = this.toasts.find((t) => t.id === id);
    if (!toast || toast.dismissing) return;

    this.clearTimer(this.autoDismissTimers, id);
    this.deadlines.delete(id);
    this.remaining.delete(id);

    this.toasts = this.toasts.map((t) =>
      t.id === id ? { ...t, dismissing: true } : t
    );

    const timer = setTimeout(() => {
      this.removalTimers.delete(id);
      this.toasts = this.toasts.filter((t) => t.id !== id);
    }, 300);
    this.removalTimers.set(id, timer);
  }

  clear() {
    this.clearTimers(this.autoDismissTimers);
    this.clearTimers(this.removalTimers);
    this.deadlines.clear();
    this.remaining.clear();
    this.toasts = [];
  }

  private startAutoDismiss(id: string, delay: number) {
    this.deadlines.set(id, Date.now() + delay);
    const timer = setTimeout(() => {
      this.autoDismissTimers.delete(id);
      this.deadlines.delete(id);
      this.dismiss(id);
    }, delay);
    this.autoDismissTimers.set(id, timer);
  }

  private setPaused(id: string, paused: boolean) {
    this.toasts = this.toasts.map((t) => (t.id === id ? { ...t, paused } : t));
  }

  private clearTimer(
    timers: Map<string, ReturnType<typeof setTimeout>>,
    id: string,
  ) {
    const timer = timers.get(id);
    if (!timer) return;

    clearTimeout(timer);
    timers.delete(id);
  }

  private clearTimers(timers: Map<string, ReturnType<typeof setTimeout>>) {
    for (const timer of timers.values()) {
      clearTimeout(timer);
    }
    timers.clear();
  }
}

export const toastStore = new ToastStore();
