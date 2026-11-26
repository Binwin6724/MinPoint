import { describe, expect, it, vi } from "vitest";
import { isBackendStarting, retryWhile } from "./retry";

const STARTING = "state not managed for field `rt` on command `bootstrap`. You must call `.manage()` before using this command";

describe("retryWhile (startup race)", () => {
  it("retries while the backend is still starting, then succeeds", async () => {
    let n = 0;
    const fn = vi.fn(async () => {
      if (++n < 4) throw STARTING;
      return "ready";
    });
    await expect(retryWhile(fn, isBackendStarting, { delayMs: 1 })).resolves.toBe("ready");
    expect(fn).toHaveBeenCalledTimes(4);
  });

  it("does not retry unrelated errors", async () => {
    const fn = vi.fn(async () => {
      throw "database is locked";
    });
    await expect(retryWhile(fn, isBackendStarting, { delayMs: 1 })).rejects.toBe("database is locked");
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("gives up after the attempt limit", async () => {
    const fn = vi.fn(async () => {
      throw STARTING;
    });
    await expect(retryWhile(fn, isBackendStarting, { attempts: 3, delayMs: 1 })).rejects.toBe(STARTING);
    expect(fn).toHaveBeenCalledTimes(3);
  });

  it("stops when cancelled", async () => {
    let cancelled = false;
    const fn = vi.fn(async () => {
      cancelled = true;
      throw STARTING;
    });
    await expect(retryWhile(fn, isBackendStarting, { delayMs: 1, isCancelled: () => cancelled })).rejects.toBe(STARTING);
    expect(fn).toHaveBeenCalledTimes(1);
  });
});
