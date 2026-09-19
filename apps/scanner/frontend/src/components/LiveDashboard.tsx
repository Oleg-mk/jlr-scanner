import { decimalText, t } from "../i18n";
import { withDecimals } from "../liveFormat";
import {
  limitKey,
  numberOf,
  starterFor,
  toneFor,
  type LimitTone,
  type LiveLimits,
} from "../liveLimits";
import type { LiveReadValue } from "../liveRead";
import { parameterName } from "../parameterNames";
import { unitLabel, withUnit } from "../units";

/**
 * The instrument panel of a live run, after the car's own cluster: the
 * speedometer on the left, the tachometer on the right, and between them
 * the screen a cluster gives to what matters now. Here that screen holds
 * whatever has crossed a limit the person set, and while nothing has, the
 * readings a mechanic looks at first — coolant, supply voltage, oil,
 * intake air. Below, as on the cluster, the outside temperature when the
 * run reads it (the owner, 2026-09-17).
 *
 * The dials are drawn to the scale of an instrument, not of a data field:
 * 260 km/h and 7 000 rpm. Each has a needle, and the needle rests at zero
 * while nothing is read; the speedometer carries the odometer in its
 * window — the highest total the mileage read found, or the total a run
 * reads — and the tachometer what the gearbox says. The caption under a
 * dial is its switch: the two parameters are read on every round unless
 * the person takes one out (the owner, 2026-09-19). A red arc on a dial
 * is the person's own alarm limit and an amber one the warning; the
 * application states no threshold of its own (ADR-0020, amendment of
 * 2026-09-17).
 */

/** A dial's parameter as the set holds it: read on every round, or not. */
export interface DialControl {
  reading: boolean;
  /** While a run is on, the set is fixed, and so is the switch. */
  disabled: boolean;
  onToggle: () => void;
}

export interface DashboardDials {
  speed: DialControl | null;
  engine: DialControl | null;
}

/** The highest running total the mileage read found, in its own unit. */
export interface Odometer {
  value: number;
  unit: string | null;
}

interface LiveDashboardProps {
  values: LiveReadValue[];
  limits: Record<string, LiveLimits>;
  /** The two dials' parameters, where the survey names them. */
  dials?: DashboardDials;
  odometer?: Odometer | null;
}

/** The dial sweeps three quarters of a turn, from bottom-left to bottom-right. */
const SWEEP = 270;
const START = -135;
const CX = 100;
const CY = 100;

/** What a mechanic looks at first, in that order. */
const KEY_READINGS: RegExp[] = [
  /coolant/i,
  /battery voltage|control module voltage|module voltage|supply voltage/i,
  /oil temperature/i,
  /oil pressure/i,
  /intake air|air intake|charge air/i,
  /fuel rail pressure|fuel pressure/i,
  /boost|manifold absolute pressure/i,
  /transmission.*temperature|gearbox.*temperature|fluid temperature/i,
];

/**
 * What the gearbox says, as the library names it: the gear the transmission
 * is in, and the selector's position beside it where a module reads one.
 * A transfer case or a park brake also speaks of gears; those are not it.
 */
const GEAR = /transmission current gear|current gear|gear commanded|gear engaged|engaged gear|actual gear|gear position|^gear information$/i;
const SELECTOR = /selector lever|lever position|transmission range|shift lever|gear selector|selected gear/i;
const NOT_THE_GEARBOX = /centre differential|center differential|park brake|transfer/i;

function effective(value: LiveReadValue, limits: Record<string, LiveLimits>): LiveLimits | undefined {
  return limits[limitKey(value)] ?? starterFor(value) ?? undefined;
}

/** A parameter that only mentions the quantity - a timeout log, a threshold - is not the quantity. */
const NOT_THE_READING = /log|timeout|event|threshold|limit|maximum|minimum|request|target|desired/i;

/**
 * The watched value a dial reads: the parameter named for the quantity
 * itself first ("Vehicle speed"), then one that carries the name without
 * being a log or a threshold about it, then any of the unit at all.
 */
function gaugeValue(values: LiveReadValue[], unit: string, pattern: RegExp): LiveReadValue | null {
  const ofUnit = values.filter((value) => value.unit === unit);
  return (
    ofUnit.find((value) => pattern.test(value.name) && !NOT_THE_READING.test(value.name)) ??
    ofUnit.find((value) => pattern.test(value.name)) ??
    ofUnit[0] ??
    null
  );
}

/** The reading as the table shows it: the number with its unit, a named state, or the raw count. */
function reading(value: LiveReadValue): string {
  if (value.value !== null) return withUnit(decimalText(value.value), value.unit);
  if (value.state !== null) return value.state;
  return value.raw !== null ? String(value.raw) : "—";
}

/** The reading without its unit: what fits a dial's window. */
function short(value: LiveReadValue): string {
  if (value.state !== null) return value.state;
  if (value.value !== null) return decimalText(value.value);
  return value.raw !== null ? String(value.raw) : "—";
}

/** A whole number with thin spaces, the way a mileage is read. */
function grouped(value: number): string {
  return Math.round(value).toLocaleString("uk-UA").replace(/\u00a0/g, "\u202f");
}

function gearboxText(values: LiveReadValue[]): string | null {
  const speaksOfIt = (pattern: RegExp) => (value: LiveReadValue) =>
    pattern.test(value.name) && !NOT_THE_GEARBOX.test(value.name);
  const selector = values.find(speaksOfIt(SELECTOR)) ?? null;
  const gear = values.find(speaksOfIt(GEAR)) ?? null;
  const parts = [selector, gear]
    .filter((value): value is LiveReadValue => value !== null)
    .map(short);
  return parts.length > 0 ? parts.join(" · ") : null;
}

function point(r: number, deg: number): { x: number; y: number } {
  const rad = (deg * Math.PI) / 180;
  return { x: CX + r * Math.sin(rad), y: CY - r * Math.cos(rad) };
}

function arc(r: number, fromDeg: number, toDeg: number): string {
  const from = point(r, fromDeg);
  const to = point(r, toDeg);
  const large = toDeg - fromDeg > 180 ? 1 : 0;
  return `M ${from.x.toFixed(2)} ${from.y.toFixed(2)} A ${r} ${r} 0 ${large} 1 ${to.x.toFixed(2)} ${to.y.toFixed(2)}`;
}

function angle(fraction: number): number {
  return START + SWEEP * Math.min(1, Math.max(0, fraction));
}

interface DialProps {
  /** What the dial shows, as the table names it. */
  name: string;
  value: LiveReadValue | null;
  max: number;
  numeralStep: number;
  tickStep: number;
  /** The numerals are the value over this: 1 000 for a tachometer marked 0…7. */
  numeralDivisor: number;
  scaleLabel: string;
  unit: string;
  limits: LiveLimits | undefined;
  /** The dial's switch, where the survey names its parameter. */
  control: DialControl | null;
  /** The small window under the hub: the odometer, or what the gearbox says. */
  window: { text: string; label: string } | null;
}

function Dial({
  name,
  value,
  max,
  numeralStep,
  tickStep,
  numeralDivisor,
  scaleLabel,
  unit,
  limits,
  control,
  window,
}: DialProps) {
  const number = value === null ? null : numberOf(value);
  const ticks: number[] = [];
  for (let at = 0; at <= max; at += tickStep) ticks.push(at);
  const warnFrom = limits?.warnHigh ?? null;
  const alarmFrom = limits?.alarmHigh ?? null;
  const tone: LimitTone = toneFor(number, limits);
  const shown = number === null ? "—" : withDecimals(number, 0);
  // The needle rests at zero while nothing is read.
  const at = angle(number === null ? 0 : number / max);
  const tail = point(-12, at);
  const tip = point(78, at);
  const standing = value === null ? (control?.reading ? t("in the set") : t("not watched")) : unit;
  return (
    <figure className={`live-dash-dial live-dash-dial--${tone}`}>
      <svg viewBox="0 0 200 200" role="img" aria-label={`${name}: ${shown} ${value === null ? "" : unit}`.trim()}>
        <circle className="live-dash-face" cx={CX} cy={CY} r={96} />
        <circle className="live-dash-rim" cx={CX} cy={CY} r={92} />
        <path className="live-dash-track" d={arc(84, START, START + SWEEP)} />
        {warnFrom !== null && warnFrom < max ? (
          <path
            className="live-dash-arc live-dash-arc--warn"
            d={arc(84, angle(warnFrom / max), angle(Math.min(alarmFrom ?? max, max) / max))}
          />
        ) : null}
        {alarmFrom !== null && alarmFrom < max ? (
          <path className="live-dash-arc live-dash-arc--alarm" d={arc(84, angle(alarmFrom / max), START + SWEEP)} />
        ) : null}
        {number !== null && at > START + 0.5 ? (
          <path className="live-dash-progress" d={arc(84, START, at)} />
        ) : null}
        {ticks.map((tick) => {
          const major = tick % numeralStep === 0;
          const deg = angle(tick / max);
          const outer = point(89, deg);
          const inner = point(major ? 79 : 84, deg);
          return (
            <line
              key={tick}
              className={major ? "live-dash-tick live-dash-tick--major" : "live-dash-tick"}
              x1={outer.x.toFixed(2)}
              y1={outer.y.toFixed(2)}
              x2={inner.x.toFixed(2)}
              y2={inner.y.toFixed(2)}
            />
          );
        })}
        {ticks
          .filter((tick) => tick % numeralStep === 0)
          .map((tick) => {
            const spot = point(66, angle(tick / max));
            return (
              <text key={tick} className="live-dash-numeral" x={spot.x.toFixed(2)} y={spot.y.toFixed(2)}>
                {tick / numeralDivisor}
              </text>
            );
          })}
        <text className="live-dash-scale" x={CX} y={62}>
          {scaleLabel}
        </text>
        <line
          className="live-dash-needle"
          x1={tail.x.toFixed(2)}
          y1={tail.y.toFixed(2)}
          x2={tip.x.toFixed(2)}
          y2={tip.y.toFixed(2)}
        />
        <circle className="live-dash-hub" cx={CX} cy={CY} r={5} />
        <text className="live-dash-reading" x={CX} y={141}>
          {shown}
        </text>
        <text className="live-dash-unit" x={CX} y={156}>
          {standing}
        </text>
        {window !== null ? (
          <g className="live-dash-window" aria-label={window.label}>
            <title>{window.label}</title>
            <rect x={62} y={164} width={76} height={16} rx={3} />
            <text x={CX} y={172.5}>
              {window.text}
            </text>
          </g>
        ) : null}
      </svg>
      <figcaption className="live-dash-caption">
        {control === null ? (
          name
        ) : (
          <button
            type="button"
            className="live-dash-caption__toggle"
            aria-pressed={control.reading}
            disabled={control.disabled}
            title={
              control.reading
                ? t("Read on every round; click to leave it out.")
                : t("Not read; click to read it on every round.")
            }
            onClick={control.onToggle}
          >
            {name}
          </button>
        )}
      </figcaption>
    </figure>
  );
}

export function LiveDashboard({
  values,
  limits,
  dials = { speed: null, engine: null },
  odometer = null,
}: LiveDashboardProps) {
  const speed = gaugeValue(values, "kph", /^vehicle speed|^road speed/i);
  const engine = gaugeValue(values, "rpm", /^engine speed/i);
  const onDials = new Set(
    [speed, engine].filter((value): value is LiveReadValue => value !== null).map(limitKey),
  );
  const rest = values.filter((value) => !onDials.has(limitKey(value)) && numberOf(value) !== null);
  // What has crossed a limit the person set, alarms before warnings.
  const crossed = rest
    .map((value) => ({ value, tone: toneFor(numberOf(value), effective(value, limits)) }))
    .filter((entry) => entry.tone === "alarm" || entry.tone === "warn")
    .sort((a, b) => (a.tone === b.tone ? 0 : a.tone === "alarm" ? -1 : 1))
    .slice(0, 4);
  // While nothing has: what a mechanic looks at first, then whatever else is read.
  const key: LiveReadValue[] = [];
  for (const pattern of KEY_READINGS) {
    const found = rest.find((value) => pattern.test(value.name) && !key.includes(value));
    if (found !== undefined) key.push(found);
  }
  for (const value of rest) {
    if (key.length >= 4) break;
    if (!key.includes(value)) key.push(value);
  }
  const cells =
    crossed.length > 0
      ? crossed
      : key.slice(0, 4).map((value) => ({ value, tone: "none" as LimitTone }));
  const ambient =
    values.find((value) => value.unit === "degC" && /ambient|outside/i.test(value.name)) ?? null;
  // The odometer: the total a run reads, else the highest the mileage read found.
  const total =
    values.find((value) => value.unit === "km" && /total distance|odometer|mileage/i.test(value.name)) ??
    null;
  const odometerText =
    total !== null
      ? reading(total)
      : odometer !== null
        ? withUnit(grouped(odometer.value), odometer.unit)
        : null;
  const gearbox = gearboxText(values);

  return (
    <section className="live-dash" aria-label={t("Instrument panel")}>
      <Dial
        name={speed === null ? t("Vehicle speed") : parameterName(speed.name)}
        value={speed}
        max={260}
        numeralStep={20}
        tickStep={10}
        numeralDivisor={1}
        scaleLabel={unitLabel("kph")}
        unit={unitLabel("kph")}
        limits={speed === null ? undefined : effective(speed, limits)}
        control={dials.speed}
        window={odometerText === null ? null : { text: odometerText, label: t("Odometer") }}
      />
      <div className="live-dash-centre">
        <h3 className="live-dash-title">{crossed.length > 0 ? t("Out of limits") : t("Key readings")}</h3>
        {cells.length > 0 ? (
          <dl className="live-dash-cells">
            {cells.map(({ value, tone }) => (
              <div className={`live-dash-cell live-dash-cell--${tone}`} key={limitKey(value)}>
                <dt>{parameterName(value.name)}</dt>
                <dd>{reading(value)}</dd>
              </div>
            ))}
          </dl>
        ) : (
          <p className="live-dash-empty">{t("Nothing is being read yet.")}</p>
        )}
      </div>
      <Dial
        name={engine === null ? t("Engine speed") : parameterName(engine.name)}
        value={engine}
        max={7000}
        numeralStep={1000}
        tickStep={250}
        numeralDivisor={1000}
        scaleLabel={t("rpm × 1000")}
        unit={unitLabel("rpm")}
        limits={engine === null ? undefined : effective(engine, limits)}
        control={dials.engine}
        window={gearbox === null ? null : { text: gearbox, label: t("Gearbox") }}
      />
      {ambient !== null ? (
        <footer className="live-dash-foot">
          <span>{reading(ambient)}</span>
        </footer>
      ) : null}
    </section>
  );
}
