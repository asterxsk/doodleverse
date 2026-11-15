# Repository rules

- Long plans, reports, and decisions belong in files under `docs/`; chat links to them.
- `docs/PRODUCT_REQUIREMENTS.md` is the product authority. `IMPLEMENTATION_PLAN.md` is the delivery plan.
- Preserve the statelessness rule: boards and drafts live only in RAM. The only persistent app-owned
  exception is AI connection records in the OS credential vault. Until that vault adapter exists,
  no AI credentials are stored at all.
- No board content, prompts, answers, credentials, or private captures in Git, tests, logs, or evidence.
  Use synthetic fixtures and sentinel strings.
- Direct-only WebRTC: never add TURN, relay, or server-routed board traffic.
- One writer per file. The orchestrator owns the root manifest, lockfile, shared contracts, and integration.
- Run `cargo fmt --all`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked`, and `cargo build --workspace --release --locked` before a checkpoint.
- Never claim a runtime check that was not run. Mock success is not provider success.
- Windows test window placement: keep interactive app runs on `\\.\DISPLAY1` (origin x=-1920) so the
  user's primary monitor stays free. Do not leave test processes running.
