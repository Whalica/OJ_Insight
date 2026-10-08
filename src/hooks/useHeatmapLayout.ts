import { useLayoutEffect, useRef, useState } from 'react';

// Keep small windows scrollable, and expand both axes together on wider panels.
// Short ranges use at least half a year of columns so their cells stay readable.
export function useHeatmapLayout(weeks: number) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) => setWidth(entry.contentRect.width));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const step = Math.max(15, (width - 60) / Math.max(26, weeks));
  return { scrollRef, step, cell: step - 3, width: 60 + weeks * step, height: 36 + 7 * step };
}
