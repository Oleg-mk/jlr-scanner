import { useState } from "react";
import type { CcfReading, CcfReadSnapshot } from "../ccf";
import { currentLanguage, parameterNote, t, useLanguage, type Language } from "../i18n";
import { StatusBadge } from "./StatusBadge";

interface CcfPanelProps {
  snapshot: CcfReadSnapshot;
  running: boolean;
  adapterReady: boolean;
  surveyed: boolean;
  /** On the bench (ADR-0020): every reading is synthetic. */
  bench?: boolean;
}

function badge(snapshot: CcfReadSnapshot, running: boolean, bench: boolean) {
  if (running) return <StatusBadge tone="pending">{t("Reading…")}</StatusBadge>;
  if (snapshot.state === "FINISHED") {
    return bench ? (
      <StatusBadge tone="pending">{t("SYNTHETIC")}</StatusBadge>
    ) : (
      <StatusBadge tone="positive">{t("Read")}</StatusBadge>
    );
  }
  return <StatusBadge tone="neutral">{t("Not started")}</StatusBadge>;
}

/**
 * SDD's text in the interface's language where SDD has it: Russian for the
 * Russian interface, English otherwise (SDD has no Ukrainian); the mnemonic
 * when SDD has no text at all.
 */
function pick(language: Language, english: string, russian: string, fallback: string): string {
  if (language === "ru" && russian !== "") return russian;
  if (english !== "") return english;
  if (russian !== "") return russian;
  return fallback;
}

function groupTitle(language: Language, row: CcfReading): string {
  return pick(language, row.groupTitleEn, row.groupTitleRu, row.group);
}

function rowTitle(language: Language, row: CcfReading): string {
  return pick(language, row.titleEn, row.titleRu, row.parameter);
}

function value(language: Language, row: CcfReading): string | null {
  if (row.valueEn === null && row.valueRu === null) return row.hex;
  return pick(language, row.valueEn ?? "", row.valueRu ?? "", row.hex ?? "");
}

export function CcfPanel({ snapshot, running, adapterReady, surveyed, bench = false }: CcfPanelProps) {
  // Subscribed to the language for re-rendering; the data texts follow the
  // language the strings follow, so the two never disagree on one screen.
  useLanguage();
  const language = currentLanguage();
  const [showHidden, setShowHidden] = useState(false);
  const rows = snapshot.readings.filter((row) => showHidden || row.display);
  const differing = new Map<string, typeof snapshot.differences>();
  for (const difference of snapshot.differences) {
    const key = `${difference.block}/${difference.parameter}`;
    differing.set(key, [...(differing.get(key) ?? []), difference]);
  }
  // One heading per group, in the order the layout gives the rows.
  const groups: Array<{ key: string; title: string; rows: CcfReading[] }> = [];
  for (const row of rows) {
    const key = `${row.block}/${row.group}`;
    const last = groups[groups.length - 1];
    if (last !== undefined && last.key === key) {
      last.rows.push(row);
    } else {
      groups.push({ key, title: groupTitle(language, row), rows: [row] });
    }
  }

  return (
    <section className="ccf-panel" aria-labelledby="ccf-title">
      <div className="section-heading section-heading--action">
        <div>
          <p className="eyebrow">{t("Configuration (CCF)")}</p>
          <h2 id="ccf-title">{t("What the car is fitted with and how it is set")}</h2>
        </div>
        {badge(snapshot, running, bench)}
      </div>
      <p className="operation-copy">
        {t(
          "The car configuration file says what the car is fitted with and how it is set: brand, market, engine, gearbox, every optional system. It is read block by block from the module the data names as its keeper and from the modules holding copies, then decoded with the data's own layout. Read-only; nothing is written anywhere.",
        )}
      </p>

      {snapshot.state !== "IDLE" ? (
        <p className="button-hint" role="status">
          {t("{asked} of {planned} block reads, {answered} answered", {
            asked: snapshot.asked,
            planned: snapshot.planned,
            answered: snapshot.answered,
          })}
        </p>
      ) : null}
      {!adapterReady ? (
        <p className="button-hint">{t("Connect and verify the adapter, or the bench, first.")}</p>
      ) : !surveyed ? (
        <p className="button-hint">{t("Survey the vehicle first: the modules to ask come from it.")}</p>
      ) : null}
      {snapshot.error !== null ? (
        <div className="error-banner" role="alert">
          <h3>{t(snapshot.error.message)}</h3>
          {snapshot.error.technicalDetails !== null ? (
            <code>{snapshot.error.technicalDetails}</code>
          ) : null}
        </div>
      ) : null}

      {snapshot.masterModule !== null && snapshot.readings.length > 0 ? (
        <p className="ccf-master">
          {t("Master copy read from {module}", { module: snapshot.masterModule })}
        </p>
      ) : null}
      {snapshot.differences.length > 0 ? (
        <p className="button-hint" role="status">
          {t("{count} parameter(s) differ between the master copy and a copy", {
            count: snapshot.differences.length,
          })}
        </p>
      ) : null}
      {snapshot.hidden > 0 ? (
        <label className="check-option">
          <input
            type="checkbox"
            checked={showHidden}
            onChange={(event) => setShowHidden(event.target.checked)}
          />
          <span>{t("Show the {count} rows SDD's editor hides", { count: snapshot.hidden })}</span>
        </label>
      ) : null}

      {groups.map((group) => (
        <div key={group.key} className="ccf-group">
          <h3 className="ccf-subtitle">{group.title}</h3>
          <table className="module-table">
            <thead>
              <tr>
                <th scope="col">{t("Setting")}</th>
                <th scope="col">{t("Block")}</th>
                <th scope="col">{t("Value")}</th>
              </tr>
            </thead>
            <tbody>
              {group.rows.map((row) => {
                const differences = differing.get(`${row.block}/${row.parameter}`) ?? [];
                const shown = value(language, row);
                return (
                  <tr key={`${row.block}-${row.parameter}`}>
                    <td>
                      <strong>{rowTitle(language, row)}</strong>
                      <div className="module-validation">{row.parameter}</div>
                    </td>
                    <td>
                      <code>{row.block}</code>
                    </td>
                    <td>
                      {shown !== null && shown !== "" ? (
                        <span className="ccf-value">{shown}</span>
                      ) : (
                        "—"
                      )}
                      {row.optionCode !== null ? (
                        <span className="module-validation"> {row.optionCode}</span>
                      ) : null}
                      {row.note !== null ? (
                        <div className="module-validation">{parameterNote(row.note)}</div>
                      ) : null}
                      {differences.map((difference) => (
                        <div
                          key={`${difference.copyModule}`}
                          className="module-validation ccf-difference"
                        >
                          {t("differs in {module}: {value}", {
                            module: difference.copyModule,
                            value: difference.copyValue ?? "—",
                          })}
                        </div>
                      ))}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      ))}

      {snapshot.readings.length > 0 ? (
        <p className="button-hint ccf-caveat">
          {t(
            "These are the values the modules hold, as they hold them. Nothing here says what a value should be; where a copy differs from the master, both are shown.",
          )}
        </p>
      ) : null}
    </section>
  );
}
