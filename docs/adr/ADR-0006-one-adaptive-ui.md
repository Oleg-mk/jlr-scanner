# ADR-0006: One adaptive UI for desktop and mobile

- Status: Accepted
- Decision: Maintain one responsive React UI codebase for Windows, later macOS, Android, iOS, and tablets.
- Reason: Product behavior and diagnostic workflows should not fork by device class.
- Consequence: Platform differences stay in native adapters and composition; mobile uses network transport.
