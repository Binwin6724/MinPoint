/**
 * Call `fn` until it succeeds. Errors matching `retryIf` are retried every `delayMs`
 * up to `attempts` times; any other error is thrown immediately.
 * Used for the first `bootstrap` call, which can run before the backend has finished
 * starting on slow machines ("state not managed").
 */
export async function retryWhile<T>(
  fn: () => Promise<T>,
  retryIf: (e: unknown) => boolean,
  { attempts = 40, delayMs = 250, isCancelled = () => false }: { attempts?: number; delayMs?: number; isCancelled?: () => boolean } = {},
): Promise<T> {
  for (let i = 0; ; i++) {
    try {
      return await fn();
    } catch (e) {
      if (isCancelled() || i >= attempts - 1 || !retryIf(e)) throw e;
      await new Promise((r) => setTimeout(r, delayMs));
    }
  }
}

export const isBackendStarting = (e: unknown) => /not managed/i.test(typeof e === "string" ? e : e instanceof Error ? e.message : String(e));
