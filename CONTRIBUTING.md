# Contributing Guidelines

These guidelines are written for human contributors and coding agents (Codex, Claude Code, etc.) who update this CLI.

## Scope and Style

- Keep changes focused and minimal for the requested behavior.
- Preserve the current CLI UX unless the request explicitly asks for a breaking change.
- Prefer updating Rust source in `src/main.rs` and tests in `test/`.

## Code Changes

- Add or update CLI flags in [`src/main.rs`](src/main.rs).
- Keep scanning/filtering behavior deterministic (sorted outputs).
- Preserve current output formatting unless the user asks for a change.

## Testing Requirements

- Every behavior change should include or update tests.
- Use existing Node test files under `test/` and follow current style.
- Run full validation before finishing:
  - `npm test`
- If tests cannot run, clearly report why.

## Versioning and Changelog

- Follow semantic versioning:
  - Patch (`x.y.Z`): bug fixes, non-breaking behavior/docs/test updates.
  - Minor (`x.Y.z`): new backward-compatible features.
  - Major (`X.y.z`): breaking CLI/API changes.
- For version bumps:
  - Update `package.json` version.
  - Update `Cargo.toml` version.
  - Update `package-lock.json` root versions to match.
- Add a `CHANGELOG.md` entry for user-visible changes.
- Put newest version at the top of `CHANGELOG.md`.

## Release Hygiene Checklist

- `npm test` passes.
- `README.md` examples/options reflect current CLI behavior.
- `CHANGELOG.md` contains the change summary.
- Version fields are consistent (`package.json`, `Cargo.toml`, and `package-lock.json`).

## Notes for Agents

- Do not silently change defaults without documenting them in both README and changelog.
- Prefer explicit flags for opt-in behavior changes (for example, `--hidden`).
- Keep command names and option descriptions concise and consistent with existing wording.
