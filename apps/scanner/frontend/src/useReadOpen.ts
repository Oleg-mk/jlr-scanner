import { useEffect, useRef, useState } from "react";

/**
 * A long read is on screen when it arrives, and goes away on one button.
 *
 * The whole-car reads — the passports, the mileage — are a table per module
 * and dozens of rows in each. Left on screen for the rest of the session
 * they bury everything under them, and the owner said so (2026-09-18). Put
 * away by default they are worse: a read that takes half a minute would
 * finish with nothing visibly happening.
 *
 * So: shown when a read arrives, and closed by the person when they have
 * seen it. Reading again brings it back, because the person asked for it
 * again.
 */
export function useReadOpen(readings: number) {
  const [open, setOpen] = useState(true);
  const seen = useRef(readings);
  useEffect(() => {
    if (readings === seen.current) return;
    seen.current = readings;
    if (readings > 0) setOpen(true);
  }, [readings]);
  return [open, setOpen] as const;
}
