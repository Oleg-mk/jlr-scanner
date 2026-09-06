import { t } from "./i18n";
import type { ModuleSurveyEntry } from "./library";
import type { ModuleReadSnapshot } from "./moduleRead";

/**
 * The vehicle network as SDD draws it — buses as lanes, modules as nodes —
 * but with every state explained. Pure functions over the survey and the
 * reads made so far; nothing here talks to the adapter.
 */

export type RouteKind = "documented" | "hypothesis" | "none";

/** How the survey placed a module relative to the adapter. */
export function moduleRoute(module: ModuleSurveyEntry): RouteKind {
  const statuses = [module.identifierRead.status, module.dtcRead.status];
  if (statuses.includes("REACHABLE")) return "documented";
  if (statuses.includes("HYPOTHESIS")) return "hypothesis";
  return "none";
}

export type NodeState =
  | "reachable"
  | "hypothesis"
  | "unreachable"
  | "not_applicable"
  | "reading"
  | "answered"
  | "declined"
  | "silent"
  | "failed";

export interface NodeStatus {
  state: NodeState;
  /** Two or three words, shown beside the node and read out with it. */
  label: string;
  /** The reason or the answer, in a sentence. */
  detail: string;
}

export function moduleReasons(module: ModuleSurveyEntry): string[] {
  const reasons = Array.from(new Set([...module.identifierRead.reasons, ...module.dtcRead.reasons]));
  if (moduleRoute(module) === "none" && isGatewayedSubNetwork(module.logicalNetwork)) {
    reasons.push(
      t(
        "Behind a gateway: SDD opens it with a routine command, which this read-only stage does not send. Planned for stage 2; the address is known.",
      ),
    );
  }
  return reasons;
}

export function nodeStatus(
  module: ModuleSurveyEntry,
  outcome: ModuleReadSnapshot | undefined,
  reading: boolean,
): NodeStatus {
  if (reading) {
    return { state: "reading", label: t("Reading…"), detail: t("A read-only request is in flight.") };
  }
  if (outcome !== undefined && outcome.state === "SUCCEEDED") {
    if (outcome.negativeResponse !== null) {
      return {
        state: "declined",
        label: t("Declined"),
        detail: t("The module answered with a negative response: {response}.", {
          response: outcome.negativeResponse,
        }),
      };
    }
    const faults = outcome.dtcs.length;
    const who = outcome.responder ?? t("the module");
    return {
      state: "answered",
      label:
        faults === 0 ? t("Answered") : faults === 1 ? t("1 fault code") : t("{count} fault codes", { count: faults }),
      detail:
        faults > 0
          ? t("Answered from {who} with {count} confirmed fault code(s).", { who, count: faults })
          : t("Answered from {who}; no confirmed fault codes.", { who }),
    };
  }
  if (outcome !== undefined && outcome.state === "FAILED") {
    if (outcome.error?.category === "NO_RESPONSE_FROM_ECU") {
      return {
        state: "silent",
        label: t("No answer"),
        detail:
          moduleRoute(module) === "hypothesis"
            ? t("No answer on the hypothesised route. Silence does not confirm it; the report records the attempt.")
            : t("No answer within the timeout. The module may be absent, asleep, or on another bus."),
      };
    }
    return {
      state: "failed",
      label: t("Failed"),
      detail: outcome.error?.message ?? t("The read failed before an answer."),
    };
  }
  switch (moduleRoute(module)) {
    case "documented":
      return {
        state: "reachable",
        label: t("Reachable"),
        detail: t("Documented route: {route}.", { route: routeText(module) }),
      };
    case "hypothesis":
      return {
        state: "hypothesis",
        label: t("Unverified route"),
        detail: t("Route is a hypothesis: {route}. A read-only answer confirms it; silence refutes it.", {
          route: routeText(module),
        }),
      };
    default: {
      const reasons = moduleReasons(module);
      const notApplicable =
        module.identifierRead.status === "NOT_APPLICABLE" &&
        module.dtcRead.status === "NOT_APPLICABLE";
      return notApplicable
        ? {
            state: "not_applicable",
            label: t("Not on this vehicle"),
            detail: reasons[0] ?? t("The data does not associate this module with the described vehicle."),
          }
        : {
            state: "unreachable",
            label: t("Not reachable"),
            detail: reasons[0] ?? t("The data does not place this module on an adapter route."),
          };
    }
  }
}

export function routeText(module: ModuleSurveyEntry): string {
  if (module.backendRoute === null) return t("no adapter route");
  const parts = [module.backendRoute];
  if (module.pins !== null) parts.push(t("pins {pins}", { pins: module.pins }));
  if (module.bitrateBps !== null) parts.push(`${module.bitrateBps / 1000} kbit/s`);
  return parts.join(" · ");
}

export type LaneKind = "documented" | "hypothesis" | "unbound" | "unplaced";

export interface Lane {
  id: string;
  name: string;
  kind: LaneKind;
  /** The adapter route text, or why there is none. */
  route: string;
  /** For a gatewayed sub-network: why it waits, in words a tester can trust. */
  note: string | null;
  modules: ModuleSurveyEntry[];
}

/**
 * SDD's sub-networks — the MOST ring, a second CAN, the newer NGI — sit
 * behind a gateway module that SDD opens with a routine command. A
 * read-only stage sends no commands, so they wait for stage 2.
 */
export function isGatewayedSubNetwork(bus: string | null) {
  return bus !== null && (/^SUB_/.test(bus) || bus === "NGI");
}

const laneOrder: Record<LaneKind, number> = {
  documented: 0,
  hypothesis: 1,
  unbound: 2,
  unplaced: 3,
};

/** Group the survey by logical bus, most useful lanes first. */
export function lanes(modules: ModuleSurveyEntry[]): Lane[] {
  const byBus = new Map<string, ModuleSurveyEntry[]>();
  for (const module of modules) {
    const key = module.logicalNetwork ?? "";
    byBus.set(key, [...(byBus.get(key) ?? []), module]);
  }
  const result: Lane[] = [];
  for (const [bus, members] of byBus) {
    const sorted = [...members].sort((left, right) => left.ecuFamily.localeCompare(right.ecuFamily));
    const documented = sorted.find((module) => moduleRoute(module) === "documented");
    const hypothesis = sorted.find((module) => moduleRoute(module) === "hypothesis");
    let kind: LaneKind;
    let route: string;
    if (bus === "") {
      kind = "unplaced";
      route = t("bus not recorded in the data");
    } else if (documented !== undefined) {
      kind = "documented";
      route = routeText(documented);
    } else if (hypothesis !== undefined) {
      kind = "hypothesis";
      route = t("unverified: {route}", { route: routeText(hypothesis) });
    } else {
      kind = "unbound";
      const reason = sorted.map(moduleReasons).find((reasons) => reasons.length > 0)?.[0];
      route = reason !== undefined ? t("not bound: {reason}", { reason }) : t("not bound to the adapter");
    }
    result.push({
      id: bus === "" ? "unplaced" : bus,
      name: bus === "" ? t("No bus") : bus,
      kind,
      route,
      note:
        kind === "unbound" && isGatewayedSubNetwork(bus)
          ? t(
              "SDD reaches this sub-network through a gateway module with a routine command. This stage only reads and sends no commands, so these modules wait for stage 2; their addresses are known and nothing else is missing.",
            )
          : null,
      modules: sorted,
    });
  }
  return result.sort(
    (left, right) => laneOrder[left.kind] - laneOrder[right.kind] || left.name.localeCompare(right.name),
  );
}

/** Modules a read-only check may be attempted on. */
export function checkableModules(
  modules: ModuleSurveyEntry[],
  includeHypothesis: boolean,
): ModuleSurveyEntry[] {
  return modules.filter((module) => {
    const route = moduleRoute(module);
    return route === "documented" || (includeHypothesis && route === "hypothesis");
  });
}
