# ADR-0002: Rust core, Tauri 2, and React

- Status: Accepted
- Decision: Implement the native core in Rust, use Tauri 2 as the shell, and React/TypeScript with Vite for presentation.
- Reason: This supports native transports and one desktop/mobile UI codebase without moving hardware or diagnostic logic into Node.js.
- Consequence: Tauri commands remain thin adapters; Electron is not used.
