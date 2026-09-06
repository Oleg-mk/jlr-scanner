import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LibraryPanel } from "./components/LibraryPanel";
import { createLibrarySnapshot, type LibrarySnapshot } from "./library";

function renderWith(snapshot: LibrarySnapshot) {
  return render(
    <LibraryPanel
      snapshot={snapshot}
      directory="C:\\library"
      busy={false}
      onDirectoryChange={() => {}}
      onLoad={() => {}}
    />,
  );
}

const issued = {
  issuedTo: "Тест Тестенко",
  issuedOn: "2026-09-05",
  issueCode: "AB12-CD34",
  validUntil: "2026-10-05",
  daysLeft: 29,
  issuer: "JLR Scanner",
};

describe("the issue stamp on a library copy", () => {
  it("says whose copy it is and until when, when the owner's signature holds", () => {
    renderWith({
      ...createLibrarySnapshot(),
      state: "LOADED",
      issue: { ...issued, integrity: "MATCHES" },
    });
    expect(
      screen.getByText(
        /Issued to Тест Тестенко on 2026-09-05, copy AB12-CD34, valid until 2026-10-05 \(29 days left\)/,
      ),
    ).toBeVisible();
    expect(screen.queryByText(/expires in/)).toBeNull();
  });

  it("asks for a new copy a week before the date", () => {
    renderWith({
      ...createLibrarySnapshot(),
      state: "LOADED",
      issue: { ...issued, daysLeft: 3, integrity: "MATCHES" },
    });
    expect(screen.getByText(/expires in 3 days/)).toBeVisible();
  });

  it("says the library was not loaded when the copy has expired", () => {
    renderWith({
      ...createLibrarySnapshot(),
      state: "FAILED",
      issue: { ...issued, daysLeft: -2, integrity: "EXPIRED" },
    });
    expect(screen.getByRole("alert")).toHaveTextContent(/has expired.*not loaded/);
  });

  it("says the library was not loaded when the folder carries no stamp", () => {
    renderWith({
      ...createLibrarySnapshot(),
      state: "FAILED",
      issue: { ...issued, issuedTo: "", issueCode: "", integrity: "NO_STAMP" },
    });
    expect(screen.getByRole("alert")).toHaveTextContent(/no issue stamp/);
  });

  it("warns when the data no longer matches the stamp", () => {
    renderWith({
      ...createLibrarySnapshot(),
      state: "FAILED",
      issue: { ...issued, integrity: "MISMATCH" },
    });
    expect(screen.getByRole("alert")).toHaveTextContent(/does not match.*not loaded/);
  });

  it("names the code the data carries when the stamp file is gone", () => {
    renderWith({
      ...createLibrarySnapshot(),
      state: "FAILED",
      issue: { ...issued, issuedTo: "", issuedOn: "", integrity: "STAMP_REMOVED" },
    });
    expect(screen.getByRole("alert")).toHaveTextContent(/AB12-CD34/);
  });

  it("says nothing about a stamp when there is none", () => {
    renderWith({ ...createLibrarySnapshot(), state: "LOADED" });
    expect(screen.queryByText(/Issued to/)).toBeNull();
  });
});
