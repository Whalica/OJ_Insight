export function visibleMonthLabels<T extends { week: number }>(months: T[], step: number): T[] {
  const visible: T[] = [];
  for (const month of months) {
    if (visible.length && (month.week - visible[visible.length - 1].week) * step < 46) {
      visible.pop();
    }
    visible.push(month);
  }
  return visible;
}
