import type { StoreActionResult } from "$lib/types/wallpaper";

interface StoreErrorState {
  error: string | null;
}

interface StoreLoadingState extends StoreErrorState {
  loading: boolean;
}

export function storeActionFailure<T = void>(
  state: StoreErrorState,
  message: string,
  cause: unknown,
): StoreActionResult<T> {
  const detail = cause instanceof Error ? cause.message : String(cause);
  const error = `${message}: ${detail}`;
  state.error = error;
  return { ok: false, error };
}

export async function runStoreTask<T>(
  state: StoreLoadingState,
  failureMessage: string,
  task: () => Promise<T>,
): Promise<StoreActionResult<T>> {
  state.loading = true;
  state.error = null;

  try {
    return { ok: true, value: await task() };
  } catch (cause) {
    return storeActionFailure<T>(state, failureMessage, cause);
  } finally {
    state.loading = false;
  }
}
