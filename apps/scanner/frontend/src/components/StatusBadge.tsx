interface StatusBadgeProps {
  children: string;
  tone: "positive" | "pending" | "negative" | "neutral";
}

export function StatusBadge({ children, tone }: StatusBadgeProps) {
  return (
    <span className={`status-badge status-badge--${tone}`}>
      <span className="status-badge__dot" aria-hidden="true" />
      {children}
    </span>
  );
}
