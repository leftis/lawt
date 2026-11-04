import { useCallback, useEffect, useRef, useState } from "react";
import { getMonth, type Photo } from "../api";

/** Month files requested at once; enough to fill a screen quickly without flooding the API. */
const MAX_IN_FLIGHT = 4;

export interface Months {
  /** Loaded months: `YYYY-MM` → photos, newest first. */
  pages: ReadonlyMap<string, readonly Photo[]>;
  /** Queues a month for loading; a no-op if it is loaded or already queued. */
  ensure: (month: string) => void;
  /** Months that failed to load, with the message. */
  errors: ReadonlyMap<string, string>;
}

/** Loads month pages on demand with de-duplication, a small concurrency limit and cleanup on unmount. */
export function useMonths(): Months {
  const [pages, setPages] = useState<ReadonlyMap<string, readonly Photo[]>>(new Map());
  const [errors, setErrors] = useState<ReadonlyMap<string, string>>(new Map());
  const requested = useRef(new Set<string>());
  const queue = useRef<string[]>([]);
  const active = useRef(0);
  const controller = useRef<AbortController | null>(null);

  const pump = useCallback(() => {
    const signal = controller.current?.signal;
    while (active.current < MAX_IN_FLIGHT && queue.current.length > 0 && signal) {
      const month = queue.current.shift();
      if (!month) break;
      active.current++;
      getMonth(month, signal)
        .then((page) => setPages((prev) => new Map(prev).set(month, page.photos)))
        .catch((err: unknown) => {
          if (signal.aborted) return;
          // Allow a retry on the next request for this month.
          requested.current.delete(month);
          setErrors((prev) => new Map(prev).set(month, err instanceof Error ? err.message : String(err)));
        })
        .finally(() => {
          active.current--;
          if (!signal.aborted) pump();
        });
    }
  }, []);

  useEffect(() => {
    const ctl = new AbortController();
    controller.current = ctl;
    const pending = requested.current;
    return () => {
      ctl.abort();
      pending.clear();
      queue.current = [];
      active.current = 0;
    };
  }, []);

  const ensure = useCallback(
    (month: string) => {
      if (requested.current.has(month)) return;
      requested.current.add(month);
      queue.current.push(month);
      pump();
    },
    [pump],
  );

  return { pages, ensure, errors };
}
