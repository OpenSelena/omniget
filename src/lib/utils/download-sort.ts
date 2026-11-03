/** Fields used to order download rows. */
export type SortableDownload = {
  id: number;
  startedAt: number;
  lastUpdateAt: number;
};

/**
 * Newest first (issue #96). Finished rows are ordered by when they last
 * changed (completion or failure time), so a download that just finished is
 * at the top and never hidden behind the "show more" pagination. Ties fall
 * back to the start time, then to the id (ids grow with time).
 */
export function sortNewestFirst<T extends SortableDownload>(items: readonly T[]): T[] {
  return [...items].sort(
    (a, b) =>
      (b.lastUpdateAt || 0) - (a.lastUpdateAt || 0) ||
      (b.startedAt || 0) - (a.startedAt || 0) ||
      b.id - a.id,
  );
}

/**
 * Newest started first. Used for rows still in progress, whose `lastUpdateAt`
 * changes on every progress tick and would make the list jump around.
 */
export function sortByStartNewestFirst<T extends SortableDownload>(items: readonly T[]): T[] {
  return [...items].sort((a, b) => (b.startedAt || 0) - (a.startedAt || 0) || b.id - a.id);
}
