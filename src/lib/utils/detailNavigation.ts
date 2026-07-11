export function indexById<T extends { id: number | string }>(
  items: T[],
  id: T["id"],
) {
  const index = items.findIndex((item) => item.id === id);
  return index >= 0 ? index : null;
}

export function nextCircularIndex(
  currentIndex: number,
  count: number,
  direction: number,
) {
  if (count <= 0) return null;
  return (currentIndex + direction + count) % count;
}
