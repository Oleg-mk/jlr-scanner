# ADR-0001: Modular architecture and separated layers

- Status: Accepted
- Decision: Keep JLR knowledge, protocols, transport adapters, diagnostic orchestration, application shell, and UI in explicit packages with one-way dependencies.
- Reason: Vehicle applicability, wire protocols, hardware access, and presentation change independently and require different validation.
- Consequence: Cross-layer shortcuts are rejected; architectural changes require an ADR.
