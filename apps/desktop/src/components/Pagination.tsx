export function Pagination({
  offset,
  limit,
  total,
  onOffsetChange,
}: {
  offset: number;
  limit: number;
  total: number;
  onOffsetChange: (offset: number) => void;
}) {
  const start = total === 0 ? 0 : offset + 1;
  const end = Math.min(offset + limit, total);

  return (
    <div className="pagination">
      <button className="btn secondary" disabled={offset === 0} onClick={() => onOffsetChange(Math.max(0, offset - limit))}>
        Previous
      </button>
      <button className="btn secondary" disabled={end >= total} onClick={() => onOffsetChange(offset + limit)}>
        Next
      </button>
      <span>
        {start}-{end} of {total}
      </span>
    </div>
  );
}
