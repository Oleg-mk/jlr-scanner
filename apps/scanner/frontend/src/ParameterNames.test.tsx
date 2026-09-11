import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { MileagePanel } from "./components/MileagePanel";
import { LANGUAGE_STORAGE_KEY, setCurrentLanguage } from "./i18n";
import { createMileageSnapshot, type MileageReading } from "./mileage";
import {
  loadParameterNames,
  parameterName,
  resetParameterNames,
  type ParameterNameClient,
} from "./parameterNames";

/**
 * The parameter names in the interface's language (ADR-0025). The table is
 * the application's own and is fetched once per language; a name it does not
 * carry is shown in SDD's English rather than guessed at, and the English is
 * what every report keeps.
 */
class Table implements ParameterNameClient {
  calls: string[] = [];

  load(language: "ukr" | "rus") {
    this.calls.push(language);
    return Promise.resolve(
      language === "ukr"
        ? { "Total distance": "Загальний пробіг" }
        : { "Total distance": "Общий пробег" },
    );
  }
}

/** Answers only when told to, so two requests can be held in flight. */
class SlowTable implements ParameterNameClient {
  private waiting = new Map<string, (table: Record<string, string>) => void>();

  load(language: "ukr" | "rus") {
    return new Promise<Record<string, string>>((resolve) => {
      this.waiting.set(language, resolve);
    });
  }

  answer(language: "ukr" | "rus") {
    const resolve = this.waiting.get(language);
    if (resolve === undefined) throw new Error(`nothing waiting for ${language}`);
    this.waiting.delete(language);
    resolve(
      language === "ukr"
        ? { "Total distance": "Загальний пробіг" }
        : { "Total distance": "Общий пробег" },
    );
  }
}

class RefusingTable implements ParameterNameClient {
  load() {
    return Promise.reject(new Error("no table"));
  }
}

function eventReading(): MileageReading {
  return {
    ecuFamily: "TCM",
    identifier: "0x1EC2",
    parameter: "Total distance",
    kind: "EVENT",
    state: "SUCCEEDED",
    value: "271400",
    unit: "km",
    number: 271_400,
    raw: 271_400,
    difference: 2_495,
    routeId: "hs-can",
    routeValidation: "SOURCE_BACKED",
    rawResponseHex: null,
    negativeResponse: null,
    note: null,
    reason: null,
  };
}

describe("the parameter names", () => {
  afterEach(() => {
    resetParameterNames();
    setCurrentLanguage("en");
    window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
  });

  it("says a name in the interface's language and leaves an unknown one in English", async () => {
    const table = new Table();
    setCurrentLanguage("uk");
    await loadParameterNames("uk", table);
    expect(table.calls).toEqual(["ukr"]);
    expect(parameterName("Total distance")).toBe("Загальний пробіг");
    // Fail closed, one name at a time: no guess, and no falling back to the
    // other language.
    expect(parameterName("Odometer store")).toBe("Odometer store");
  });

  it("asks for no table in English, and asks once per language", async () => {
    const table = new Table();
    setCurrentLanguage("en");
    await loadParameterNames("en", table);
    expect(table.calls).toEqual([]);
    expect(parameterName("Total distance")).toBe("Total distance");

    setCurrentLanguage("ru");
    await loadParameterNames("ru", table);
    await loadParameterNames("ru", table);
    expect(table.calls).toEqual(["rus"]);
    expect(parameterName("Total distance")).toBe("Общий пробег");
  });

  it("drops a table that arrives after the language has moved on", async () => {
    // Two requests in flight, the first slower. The interface is in Russian
    // by the time either lands, so the Ukrainian answer must not be shown.
    const slow = new SlowTable();
    setCurrentLanguage("uk");
    const first = loadParameterNames("uk", slow);
    setCurrentLanguage("ru");
    const second = loadParameterNames("ru", slow);
    slow.answer("rus");
    expect(await second).toBe(true);
    slow.answer("ukr");
    expect(await first).toBe(false);
    expect(parameterName("Total distance")).toBe("Общий пробег");
  });

  it("leaves every name English when the table will not load", async () => {
    setCurrentLanguage("uk");
    await loadParameterNames("uk", new RefusingTable());
    expect(parameterName("Total distance")).toBe("Total distance");
  });

  it("reaches the table on screen", async () => {
    setCurrentLanguage("uk");
    await loadParameterNames("uk", new Table());
    render(
      <MileagePanel
        snapshot={{
          ...createMileageSnapshot(),
          state: "FINISHED",
          planned: 1,
          asked: 1,
          answered: 1,
          highest: 268_905,
          highestModule: "ABS",
          unit: "km",
          readings: [eventReading()],
        }}
        running={false}
        adapterReady
        surveyed
      />,
    );
    expect(screen.getByText("Загальний пробіг")).toBeInTheDocument();
    expect(screen.queryByText("Total distance")).not.toBeInTheDocument();
  });
});
