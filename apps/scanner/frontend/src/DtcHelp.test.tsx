import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { DtcHelp } from "./components/DtcHelp";
import { resetHelpLanguage } from "./helpLanguage";
import { LanguageContext, setCurrentLanguage } from "./i18n";
import type { DtcSummary } from "./moduleRead";

/**
 * The help under a code is SDD's own text in English or Russian (ADR-0034):
 * a Ukrainian interface reads Russian by default, can switch to English, and
 * is offered nothing else. A library issued without the Russian pack shows
 * English and says so.
 */
function code(helpTexts: Record<string, string[]>): DtcSummary {
  return {
    code: "P0300",
    codeBytes: [0x03, 0x00],
    failureType: "00",
    status: "0x2F",
    description: "Random / multiple cylinder misfire detected",
    descriptionScope: null,
    failureTypeText: null,
    failureTypeTexts: {},
    descriptionTexts: {},
    help: ["Possible causes:", "Ignition, fuel or compression"],
    helpTexts,
    helpNote: null,
  } as unknown as DtcSummary;
}

function show(dtc: DtcSummary, language: "uk" | "en" | "ru") {
  setCurrentLanguage(language);
  return render(
    <LanguageContext.Provider value={language}>
      <DtcHelp dtc={dtc} />
    </LanguageContext.Provider>,
  );
}

describe("the help under a code, in SDD's own languages", () => {
  beforeEach(() => resetHelpLanguage());
  afterEach(() => {
    cleanup();
    setCurrentLanguage("en");
  });

  it("reads Russian to a Ukrainian interface and offers only English beside it", () => {
    show(code({ rus: ["Возможные причины:", "Зажигание, топливо или компрессия"] }), "uk");
    expect(screen.getByText("Зажигание, топливо или компрессия")).toBeInTheDocument();
    expect(screen.queryByText("Ignition, fuel or compression")).not.toBeInTheDocument();
    // Two choices and no third: SDD has no Ukrainian and we write none.
    const buttons = screen.getAllByRole("button");
    expect(buttons.map((button) => button.textContent)).toEqual(["Англійська", "Російська"]);
    expect(screen.getByRole("button", { name: "Російська" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("switches to English on request and remembers it", () => {
    show(code({ rus: ["Возможные причины:", "Зажигание, топливо или компрессия"] }), "uk");
    fireEvent.click(screen.getByRole("button", { name: "Англійська" }));
    expect(screen.getByText("Ignition, fuel or compression")).toBeInTheDocument();
    expect(screen.queryByText("Зажигание, топливо или компрессия")).not.toBeInTheDocument();
    cleanup();
    // A fresh render keeps the choice.
    show(code({ rus: ["Возможные причины:", "Зажигание, топливо или компрессия"] }), "uk");
    expect(screen.getByText("Ignition, fuel or compression")).toBeInTheDocument();
  });

  it("reads English to an English interface by default", () => {
    show(code({ rus: ["Возможные причины:", "Зажигание, топливо или компрессия"] }), "en");
    expect(screen.getByText("Ignition, fuel or compression")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "English" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("shows English and says so when the library has no Russian for the screen", () => {
    show(code({}), "uk");
    expect(screen.getByText("Ignition, fuel or compression")).toBeInTheDocument();
    expect(
      screen.getByText("У цій бібліотеці немає російського тексту для цього."),
    ).toBeInTheDocument();
  });
});
