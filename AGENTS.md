# Pocket Studio Engineering Guide

## Product

Pocket Studio is the desktop bridge for the Pocket ecosystem. It discovers devices over USB or the network, identifies their platform, prepares them for Pocket workflows, and installs compatible software.

Device preparation can change a connected device. Every destructive or privileged step must be explicit, observable, and initiated by the user. Never hide root, jailbreak, erase, unlock, or firmware actions behind discovery or another read-only operation.

## Stack

- Tauri 2 with Rust for the native application layer
- Vue 3, TypeScript, Vite, and Tailwind CSS 4 for the webview
- Vue I18n for all user-facing strings
- pnpm for JavaScript dependencies and scripts
- `tracing` for Rust diagnostics
- `lefthook` for local quality gates

## Architecture

All device I/O belongs in `src-tauri`. The frontend may invoke typed Tauri commands, but it must not access USB devices, network sockets, subprocesses, or platform APIs directly.

Rust modules follow these boundaries:

- `domain`: device, application, and workflow models plus pure rules
- `application`: use cases that coordinate domain behavior
- `infrastructure`: USB/network transports, platform adapters, persistence, and external tools
- `commands`: the small serialization boundary exposed to Tauri

Keep domain and application code platform-neutral. Platform-specific code must live behind traits in clearly named `macos`, `linux`, and `windows` modules. When one platform needs a specialized implementation, define the behavior for all three desktop targets before merging.

## Rust conventions

- Libraries and domain modules expose focused errors derived with `thiserror`.
- The binary/application composition root uses `anyhow` for contextual startup failures.
- Prefer ownership and types that make invalid states unrepresentable over repeated runtime validation.
- Avoid speculative fallbacks, duplicate parameter checks, and broad catch-all branches.
- Keep Tauri commands thin; business behavior belongs in application services.
- Use `tracing` consistently:
  - `error`: an operation failed and cannot continue
  - `warn`: a recoverable or user-actionable problem
  - `info`: lifecycle milestones and user-requested operations
  - `debug`: identifiers, decisions, and diagnostic context
  - `trace`: protocol-level details with no secrets or personal data
- Never log credentials, pairing records, private keys, device contents, or full sensitive identifiers.

## Frontend conventions

- Use Vue Single-File Components with `<script setup lang="ts">`.
- Keep native calls in typed gateway modules, never inline in components.
- Put all user-facing text in locale files. English is the fallback locale.
- Support light, dark, and system theme preferences through design tokens.
- Do not add a page, state library, or component abstraction before a concrete feature needs it.

## Quality

Run these commands before committing:

```sh
pnpm check
pnpm test:native
```

`lefthook` runs formatting and lint gates for staged work. Do not bypass it. Tests should cover important rules and failure-prone boundaries; avoid tests that only mirror implementation details.

## Git

Use Conventional Commits with small, reviewable changes:

```text
<type>(<scope>): <imperative summary>
```

Common types are `feat`, `fix`, `refactor`, `test`, `docs`, `build`, and `chore`. Keep generated dependency lockfile changes in the commit that introduces or updates the dependency.

After completing each coherent feature and passing the required quality checks, immediately create a Conventional Commit before starting the next feature. Do not wait for a separate user request to commit. Keep related implementation and tests together, and keep unrelated completed features in separate commits; avoid file-by-file or unfinished intermediate commits.
