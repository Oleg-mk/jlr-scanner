import { useEffect, useState } from "react";
import { t, useLanguage } from "../i18n";
import type { LibraryIssue } from "../library";
import type { LibrarySnapshot } from "../library";
import { StatusBadge } from "./StatusBadge";

/** Days before the date at which the panel starts asking for a new copy. */
const RENEWAL_NOTICE_DAYS = 7;

/**
 * What the stamp on the copy says (ADR-0019). Under MATCHES the library
 * loaded and the copy is named; under every other value the application
 * refused the folder, and the notice says why and what to do.
 */
function issueNotice(issue: LibraryIssue) {
  const values = {
    name: issue.issuedTo,
    date: issue.issuedOn,
    code: issue.issueCode,
    until: issue.validUntil,
    days: issue.daysLeft,
  };
  switch (issue.integrity) {
    case "MATCHES":
      return (
        <>
          <p className="library-issue">
            {t(
              "Issued to {name} on {date}, copy {code}, valid until {until} ({days} days left). This copy is personal; every report carries it.",
              values,
            )}
          </p>
          {issue.daysLeft <= RENEWAL_NOTICE_DAYS ? (
            <p className="library-issue library-issue--warning" role="status">
              {t("This copy expires in {days} days; ask for a new one before then.", values)}
            </p>
          ) : null}
        </>
      );
    case "EXPIRED":
      return (
        <p className="library-issue library-issue--warning" role="alert">
          {t(
            "This copy, issued to {name} until {until}, has expired. The library was not loaded; ask for a new copy.",
            values,
          )}
        </p>
      );
    case "NO_STAMP":
      return (
        <p className="library-issue library-issue--warning" role="alert">
          {t(
            "This folder carries no issue stamp. The application loads only a copy issued to a named person; ask for one.",
          )}
        </p>
      );
    case "UNSIGNED":
      return (
        <p className="library-issue library-issue--warning" role="alert">
          {t("The stamp of this copy is not signed. The library was not loaded; ask for a new copy.")}
        </p>
      );
    case "BAD_SIGNATURE":
      return (
        <p className="library-issue library-issue--warning" role="alert">
          {t("The stamp's signature is not the owner's. The library was not loaded; ask for a new copy.")}
        </p>
      );
    case "MISMATCH":
      return (
        <p className="library-issue library-issue--warning" role="alert">
          {t(
            "The data does not match its stamp: issued to {name}, copy {code}. The library was not loaded; ask for a new copy.",
            values,
          )}
        </p>
      );
    case "STAMP_REMOVED":
      return (
        <p className="library-issue library-issue--warning" role="alert">
          {t(
            "The stamp file is missing; the data carries copy {code}. The library was not loaded; ask for a new copy.",
            values,
          )}
        </p>
      );
    default:
      return null;
  }
}

interface LibraryPanelProps {
  snapshot: LibrarySnapshot;
  directory: string;
  busy: boolean;
  onDirectoryChange: (directory: string) => void;
  /** Present when the shell can open a native folder dialog. */
  onChooseDirectory?: () => void;
  onLoad: () => void;
}

function badge(snapshot: LibrarySnapshot) {
  switch (snapshot.state) {
    case "LOADED":
      return <StatusBadge tone="positive">{t("Loaded")}</StatusBadge>;
    case "PARTIALLY_LOADED":
      return <StatusBadge tone="pending">{t("Partially loaded")}</StatusBadge>;
    case "FAILED":
      return <StatusBadge tone="negative">{t("Failed")}</StatusBadge>;
    default:
      return <StatusBadge tone="neutral">{t("Built-in data only")}</StatusBadge>;
  }
}

export function LibraryPanel({
  snapshot,
  directory,
  busy,
  onDirectoryChange,
  onChooseDirectory,
  onLoad,
}: LibraryPanelProps) {
  useLanguage();
  // Seconds since the load began. A disabled button is not enough on a slow
  // machine: the owner's 2016 Mac read a 177 MB library with nothing moving
  // on screen, and he took it for a hang (2026-09-10).
  const [elapsed, setElapsed] = useState(0);
  useEffect(() => {
    if (!busy) {
      setElapsed(0);
      return;
    }
    const started = Date.now();
    setElapsed(0);
    const timer = window.setInterval(
      () => setElapsed(Math.round((Date.now() - started) / 1000)),
      1000,
    );
    return () => window.clearInterval(timer);
  }, [busy]);
  return (
    <section className="library-panel" aria-labelledby="library-title">
      <div className="section-heading section-heading--action">
        <div>
          <p className="eyebrow">{t("Diagnostic data")}</p>
          <h2 id="library-title">{t("Data library")}</h2>
        </div>
        {badge(snapshot)}
      </div>

      <p className="operation-copy">
        {t(
          "The application reads vehicle definitions from a directory of exported data manifests. SDD itself is never required at run time.",
        )}
      </p>

      <div className="field-row">
        <label className="field field--wide">
          <span>{t("Directory of exported manifests")}</span>
          <input
            type="text"
            name="libraryDirectory"
            value={directory}
            placeholder="C:\\Users\\you\\jlr-scanner-library"
            onChange={(event) => onDirectoryChange(event.target.value)}
          />
        </label>
        {onChooseDirectory ? (
          <button
            className="button button--secondary"
            type="button"
            onClick={onChooseDirectory}
            disabled={busy}
          >
            {t("Choose folder…")}
          </button>
        ) : null}
        <button
          className="button button--secondary"
          type="button"
          onClick={onLoad}
          disabled={busy || directory.trim() === ""}
        >
          {busy ? t("Working…") : t("Load library")}
        </button>
      </div>

      {busy ? (
        <p className="library-status" role="status">
          {t("Reading the library…")} {elapsed}&nbsp;{t("sec")}
          <br />
          <span className="button-hint">
            {t(
              "A large library takes seconds on a fast machine and tens of seconds on an old one. Nothing is wrong; wait for the count to stop.",
            )}
          </span>
        </p>
      ) : (
        <p className="library-status" role="status">
          {snapshot.message}
        </p>
      )}
      {snapshot.issue !== null ? issueNotice(snapshot.issue) : null}
      {snapshot.failures.length > 0 ? (
        <ul className="library-failures">
          {snapshot.failures.map((failure) => (
            <li key={failure.file}>
              <code>{failure.file}</code>: {failure.message}
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}
