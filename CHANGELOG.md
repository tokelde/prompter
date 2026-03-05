# Changelog

## 0.5.0

* Add default verbose completion summary (`files_found`, `files_included`, output target, forced includes)
* Add `--quiet` / `-q` to hide verbose completion output
* Add extension filtering with `--find-format` and `--exclude-format`
* Add `--include` to force specific files into snapshot, including files ignored by `.gitignore`
* Add short flags for new filters and includes: `-i`, `-F`, and `-E`

## 0.4.0

* Rewrite core CLI implementation in Rust
* Keep npm distribution while moving execution to a Rust binary
* Replace TypeScript module tests with CLI integration tests for Rust-backed behavior
* Remove JavaScript CLI wrapper and expose a direct Rust binary in npm `bin`
* Build/copy Rust binary using npm shell scripts (`cargo build --release` -> `bin/prompter`)
* Update tests to execute the Rust binary directly

## 0.3.1

* Fix default output reruns by excluding the output file from scanned inputs
* Ensure the default `prompter-output.md` is replaced cleanly on subsequent runs
* Add automated regression test for default output self-inclusion behavior

## 0.3.0

* Exclude hidden files (dotfiles) by default when scanning repository files
* Add `--hidden` flag to include hidden files on demand
* Add `CONTRIBUTING.md` with maintenance guidelines for future contributors and coding agents

## 0.2.0

* Add `--exclude` / `-e` to ignore files, folders, and globs (repeatable or comma-separated)
* Add `--top-prompt` / `-p` to prepend custom prompt text to generated output
* Add `--raw` / `-r` to print snapshot to stdout without creating a file
* Update README usage examples to use `npx @tokelde/prompter`
* Add automated tests for exclude filtering and CLI raw/top-prompt behavior

## 0.1.0

* Initial release
* Supports --contains
* Respects .gitignore
* Markdown output
