interface StatusBadgeProps {
  children: string;
  /** `bench` is the bench's own colour (ADR-0020): the badge carries the
   *  state the bands used to, since the owner withdrew them (2026-09-18). */
  tone: "positive" | "pending" | "negative" | "neutral" | "bench";
  /** A number set in a circle on the right, with its own label: the bench scenario. */
  mark?: { value: string | number; label: string };
}

export function StatusBadge({ children, tone, mark }: StatusBadgeProps) {
  return (
    <span className={`status-badge status-badge--${tone}`}>
      <span className="status-badge__dot" aria-hidden="true" />
      <span className="status-badge__text">{children}</span>
      {mark !== undefined ? (
        <span className="status-badge__mark" aria-label={mark.label} title={mark.label}>
          {mark.value}
        </span>
      ) : null}
    </span>
  );
}
