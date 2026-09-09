import { t, useLanguage } from "../i18n";
import type {
  VehicleCatalogueSnapshot,
  VehicleDescription,
  VehicleSurveySnapshot,
  VinDecodeSnapshot,
} from "../library";
import { markerLabel } from "../library";

interface VehicleCardProps {
  vehicle: VehicleDescription;
  catalogue: VehicleCatalogueSnapshot;
  survey: VehicleSurveySnapshot | null;
  busy: boolean;
  libraryReady: boolean;
  vin: string;
  vinDecode: VinDecodeSnapshot | null;
  decodingVin: boolean;
  onVinChange: (vin: string) => void;
  onDecodeVin: () => void;
  onVehicleChange: (vehicle: VehicleDescription) => void;
  onSurvey: () => void;
}

function optionalText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}


export function VehicleCard({
  vehicle,
  catalogue,
  survey,
  busy,
  libraryReady,
  vin,
  vinDecode,
  decodingVin,
  onVinChange,
  onDecodeVin,
  onVehicleChange,
  onSurvey,
}: VehicleCardProps) {
  useLanguage();
  const canSurvey = libraryReady && !busy && vehicle.vehicleProgram.trim() !== "";
  const programme = catalogue.programmes.find((entry) => entry.program === vehicle.vehicleProgram);
  const hasCatalogue = catalogue.programmes.length > 0;
  const marker = programme?.markers.find((entry) => entry.marker === vehicle.yearBreakpoint);

  const fields = hasCatalogue ? (
    <>
      <label className="field">
        <span>{t("Programme")}</span>
        <select
          name="vehicleProgram"
          value={vehicle.vehicleProgram}
          onChange={(event) =>
            onVehicleChange({
              ...vehicle,
              vehicleProgram: event.target.value,
              yearBreakpoint: null,
              modelYear: null,
              powertrain: null,
              variant: null,
            })
          }
        >
          <option value="">{t("Choose a programme")}</option>
          {catalogue.programmes.map((entry) => (
            <option key={entry.program} value={entry.program}>
              {entry.program}
            </option>
          ))}
        </select>
      </label>
      <label className="field">
        <span>{t("Model years")}</span>
        <select
          name="yearBreakpoint"
          value={vehicle.yearBreakpoint ?? ""}
          disabled={programme === undefined}
          onChange={(event) => {
            const chosen = programme?.markers.find((entry) => entry.marker === event.target.value);
            onVehicleChange({
              ...vehicle,
              yearBreakpoint: chosen?.marker ?? null,
              modelYear: chosen?.modelYearFrom ?? null,
            });
          }}
        >
          <option value="">{t("Choose")}</option>
          {(programme?.markers ?? []).map((entry) => (
            <option key={entry.marker} value={entry.marker}>
              {markerLabel(entry)}
            </option>
          ))}
        </select>
      </label>
      <label className="field">
        <span>{t("Engine")}</span>
        <select
          name="powertrain"
          value={vehicle.powertrain ?? ""}
          disabled={programme === undefined}
          onChange={(event) =>
            onVehicleChange({ ...vehicle, powertrain: optionalText(event.target.value) })
          }
        >
          <option value="">{t("Not stated")}</option>
          {(programme?.powertrains ?? []).map((entry) => (
            <option key={entry} value={entry}>
              {entry}
            </option>
          ))}
        </select>
      </label>
      {(programme?.variants ?? []).length > 0 ? (
        <label className="field">
          <span>{t("Engine variant")}</span>
          <select
            name="variant"
            value={vehicle.variant ?? ""}
            disabled={programme === undefined}
            onChange={(event) =>
              onVehicleChange({ ...vehicle, variant: optionalText(event.target.value) })
            }
          >
            <option value="">{t("Not stated")}</option>
            {(programme?.variants ?? []).map((entry) => (
              <option key={entry} value={entry}>
                {entry}
              </option>
            ))}
          </select>
        </label>
      ) : null}
    </>
  ) : (
    <>
      <label className="field">
        <span>{t("Programme")}</span>
        <input
          type="text"
          name="vehicleProgram"
          value={vehicle.vehicleProgram}
          placeholder="X250"
          onChange={(event) => onVehicleChange({ ...vehicle, vehicleProgram: event.target.value })}
        />
      </label>
      <label className="field">
        <span>{t("Model year")}</span>
        <input
          type="number"
          name="modelYear"
          value={vehicle.modelYear ?? ""}
          placeholder="2010"
          onChange={(event) =>
            onVehicleChange({
              ...vehicle,
              modelYear: event.target.value === "" ? null : Number(event.target.value),
            })
          }
        />
      </label>
      <label className="field">
        <span>{t("SDD breakpoint marker")}</span>
        <input
          type="text"
          name="yearBreakpoint"
          value={vehicle.yearBreakpoint ?? ""}
          placeholder="MY10"
          onChange={(event) =>
            onVehicleChange({ ...vehicle, yearBreakpoint: optionalText(event.target.value) })
          }
        />
      </label>
      <label className="field">
        <span>{t("Engine")}</span>
        <input
          type="text"
          name="powertrain"
          value={vehicle.powertrain ?? ""}
          placeholder={t("as SDD names it")}
          onChange={(event) =>
            onVehicleChange({ ...vehicle, powertrain: optionalText(event.target.value) })
          }
        />
      </label>
    </>
  );

  return (
    <section className="vehicle-card" aria-labelledby="vehicle-card-title">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{t("Vehicle")}</p>
          <h2 id="vehicle-card-title">
            {vehicle.vehicleProgram.trim() === "" ? t("Not chosen yet") : vehicle.vehicleProgram}
          </h2>
        </div>
      </div>
      {vehicle.vehicleProgram.trim() !== "" ? (
        <dl className="detail-list detail-list--compact vehicle-summary">
          <div>
            <dt>{t("Model years")}</dt>
            <dd>{marker !== undefined ? markerLabel(marker) : (vehicle.modelYear ?? "—")}</dd>
          </div>
          <div>
            <dt>{t("Engine")}</dt>
            <dd>{vehicle.powertrain ?? t("not stated")}</dd>
          </div>
        </dl>
      ) : null}
      <p className="operation-copy">
        {hasCatalogue
          ? t("Or choose the car from what the loaded data describes. A decoded VIN pre-selects it; confirm the engine.")
          : t("Until a library is loaded, describe the vehicle as SDD names it.")}
      </p>
      <div className="vehicle-column vehicle-column--vin">
      <div className="vin-row">
        <label className="field field--wide">
          <span>{t("VIN")}</span>
          <input
            type="text"
            name="vin"
            value={vin}
            placeholder={t("17 characters from the plate or the registration")}
            autoComplete="off"
            spellCheck={false}
            onChange={(event) => onVinChange(event.target.value)}
          />
        </label>
        <button
          className="button button--secondary"
          type="button"
          onClick={onDecodeVin}
          disabled={decodingVin || vin.replace(/[\s-]/g, "").length !== 17}
        >
          {decodingVin ? t("Decoding…") : t("Decode VIN")}
        </button>
      </div>
      {vinDecode !== null ? (
        <div className="vin-result" role="status">
          <p className={vinDecode.valid ? "vin-message" : "vin-message vin-message--invalid"}>
            {vinDecode.message}
          </p>
          {vinDecode.attributes.length > 0 ? (
            <dl className="detail-list detail-list--compact">
              {vinDecode.attributes.map((attribute) => (
                <div key={attribute.name}>
                  <dt>{attribute.name}</dt>
                  <dd>{attribute.value}</dd>
                </div>
              ))}
            </dl>
          ) : null}
          {vinDecode.tableVersion !== null ? (
            <p className="module-validation">{vinDecode.tableVersion}</p>
          ) : null}
        </div>
      ) : null}
      </div>
      <div className="vehicle-column vehicle-column--describe">
      <div className="field-stack">{fields}</div>
      <button
        className="button button--primary"
        type="button"
        onClick={onSurvey}
        disabled={!canSurvey}
      >
        {busy ? t("Working…") : t("Survey modules")}
      </button>
      </div>
      {survey !== null ? (
        <ul className="survey-stats" aria-label={t("Survey summary")}>
          <li>
            <strong>{survey.modules.length}</strong> {t("modules known")}
          </li>
          <li>
            <strong>{survey.reachable}</strong> {t("reachable")}
          </li>
          <li>
            <strong>{survey.hypothesis}</strong> {t("on unverified routes")}
          </li>
          <li>
            <strong>{survey.unreachable}</strong> {t("not reachable")}
          </li>
        </ul>
      ) : null}
    </section>
  );
}
