import { useEffect, useState } from "react";
import { t, useLanguage } from "../i18n";
import { bodyImageUrl, type BodyType } from "../vehicleBody";
import type { SessionStep } from "../sessionReport";

export interface RailVehicle {
  title: string;
  details: string[];
  /** The body the car is drawn as, when known; its image is the fallback for `image`. */
  body: BodyType | null;
  /** The owner's picture of this model, when there is one. */
  image: string | null;
}

interface SessionPanelProps {
  steps: SessionStep[];
  /** Bring the section that holds this step into view. */
  onJump: (stepId: SessionStep["id"]) => void;
  /** The vehicle of the session, once described; drawn at the foot of the rail. */
  vehicle: RailVehicle | null;
}

/**
 * The owner's picture of the model; failing that, the image for the body
 * type; failing that, the drawn mark. A source that does not load is
 * skipped for the next one.
 */
function VehicleArt({ image, body }: { image: string | null; body: BodyType | null }) {
  const sources = [image, body === null ? null : bodyImageUrl(body)].filter(
    (source): source is string => source !== null,
  );
  const [failed, setFailed] = useState<string[]>([]);
  useEffect(() => {
    setFailed([]);
  }, [image, body]);
  const source = sources.find((candidate) => !failed.includes(candidate));
  if (source !== undefined) {
    return (
      <img
        className="rail-vehicle-image"
        src={source}
        alt=""
        onError={() => setFailed((known) => [...known, source])}
      />
    );
  }
  return <VehicleMark />;
}

/**
 * The mark shown where the vehicle's image will be: a ring with two
 * mirrored hooks, after the owner's emblem. Grey until a vehicle is
 * chosen, in the accent colour once it is.
 */
function VehicleMark() {
  return (
    <svg className="rail-vehicle-mark" viewBox="0 0 120 120" aria-hidden="true">
      <circle cx="60" cy="60" r="50" />
      <path d="M53 26v50a12 12 0 0 1-12 12h-4" />
      <path d="M67 94V44a12 12 0 0 1 12-12h4" />
    </svg>
  );
}

const stateLabel: Record<SessionStep["state"], string> = {
  done: "Done",
  next: "Next",
  todo: "To do",
};

export function SessionPanel({ steps, onJump, vehicle }: SessionPanelProps) {
  useLanguage();
  const next = steps.find((step) => step.state === "next");
  return (
    <nav className="session-panel flow-rail" aria-labelledby="session-title">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{t("One session, one report")}</p>
          <h2 id="session-title">{t("Session")}</h2>
        </div>
      </div>
      <p className="operation-copy">
        {next
          ? t("Next: {title} — {hint}", { title: next.title.toLowerCase(), hint: next.hint })
          : t("Everything recorded. Save the report and send it with the tester programme.")}
      </p>
      <ol className="session-steps">
        {steps.map((step, index) => (
          <li key={step.id} className={`session-step session-step--${step.state}`}>
            <button
              type="button"
              className="session-step-button"
              onClick={() => onJump(step.id)}
              title={t("Go to step {index}", { index: index + 1 })}
            >
              <span className="session-step-index" aria-hidden="true">
                {index + 1}
              </span>
              <span className="session-step-body">
                <span className="session-step-title">
                  {step.title}
                  {step.optional ? (
                    <span className="session-step-optional"> {t("optional")}</span>
                  ) : null}
                </span>
                <span className="session-step-hint">{step.hint}</span>
              </span>
              <span
                className="session-step-state"
                aria-label={`${step.title}: ${stateLabel[step.state]}`}
              >
                {t(stateLabel[step.state])}
              </span>
            </button>
          </li>
        ))}
      </ol>
      <div
        className={vehicle ? "rail-vehicle" : "rail-vehicle rail-vehicle--empty"}
        aria-live="polite"
      >
        <VehicleArt image={vehicle?.image ?? null} body={vehicle?.body ?? null} />
        {vehicle ? (
          <>
            <strong className="rail-vehicle-title">{vehicle.title}</strong>
            {vehicle.details.map((detail) => (
              <span className="rail-vehicle-detail" key={detail}>
                {detail}
              </span>
            ))}
          </>
        ) : (
          <span className="rail-vehicle-detail">{t("Not chosen yet")}</span>
        )}
      </div>
    </nav>
  );
}
