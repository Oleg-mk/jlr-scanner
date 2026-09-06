# ADR-0007: Offline evidence-backed read-only diagnostic execution

- Status: Accepted
- Decision: Keep the existing F4 `diagnostics-core` UDS-specific. Add a sibling
  generic SAE J1979 codec and an application-layer `diagnostic-execution`
  bridge that compiles only typed read-only intent from an F6 RESOLVED plan and
  executes only through replay or simulator frame sources. Extend the offline
  simulator with a protocol-neutral ISO-TP payload seam while preserving its
  existing typed UDS API.
- Reason: SAE J1979 Mode 09 is not UDS, but both protocols legitimately consume
  the existing protocol-neutral ISO-TP and read-only CAN-frame source layers.
  Generalizing the UDS result/request types would mix application protocols,
  while coupling protocol crates or transport to F5/F6 knowledge would violate
  the primary architecture invariant.
- Consequence: `diagnostic-environment` remains data-only; `obd-j1979` depends
  on neither knowledge nor transport; `diagnostic-execution` is the one-way
  plan-to-protocol boundary; raw payload bytes remain internal to protocol and
  offline fixture plumbing; live/Mongoose sources, arbitrary TX, discovery,
  write/control/security intents, and frontend APIs remain unavailable.
