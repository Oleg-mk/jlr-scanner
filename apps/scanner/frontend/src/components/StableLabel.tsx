interface StableLabelProps {
  /** The wording shown now. */
  current: string;
  /** Every wording the control can show, so it is as wide as the widest. */
  all: string[];
}

/**
 * A label that changes with a state without moving anything: every wording
 * the control can show is laid in the same cell, the current one visible
 * and the others there but unseen, so the control keeps the width of its
 * widest word and the word changes in place, centred (the owner,
 * 2026-09-19: «вибери більший розмір і він завжди, напис міняємо по
 * центру»). The unseen wordings are hidden from assistive technology too,
 * so the control's name is the word it shows.
 */
export function StableLabel({ current, all }: StableLabelProps) {
  const texts = all.includes(current) ? all : [...all, current];
  return (
    <span className="stable-label">
      {texts.map((text) => (
        <span
          key={text}
          className={text === current ? "stable-label__on" : "stable-label__off"}
          aria-hidden={text !== current}
        >
          {text}
        </span>
      ))}
    </span>
  );
}

/** The two-state case: the wording while the state is on, and while it is off. */
export function SwapLabel({ on, whenOn, whenOff }: { on: boolean; whenOn: string; whenOff: string }) {
  return <StableLabel current={on ? whenOn : whenOff} all={[whenOn, whenOff]} />;
}
