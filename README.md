# prompter

> Turn an entire repository into one clean Markdown snapshot for LLM prompts.

`prompter` scans your project, respects ignore rules, and emits a structured file-by-file dump optimized for review, refactor, or debugging prompts.

## Basic Usage

```bash
npx @tokelde/prompter
npx @tokelde/prompter app/services
npx @tokelde/prompter -c "createUser"
npx @tokelde/prompter -p "Create a technical documentation for authentication."
```

## Advanced Usage

```bash
npx @tokelde/prompter -o snapshot.md
npx @tokelde/prompter src/server.ts src/routes/auth.ts package.json
npx @tokelde/prompter -e dist -e "README.md,.gitignore"
npx @tokelde/prompter -F py -E js
npx @tokelde/prompter -i .env.local
npx @tokelde/prompter --hidden
npx @tokelde/prompter --raw
npx @tokelde/prompter --quiet
```

## Options

- `[path-or-file ...]`: Optional positional inputs.
  - No value: defaults to current directory (`.`).
  - One directory path (relative or absolute): scans that folder and applies nested ignore rules.
  - One or more file paths (relative and/or absolute): includes only those files.
- `-e, --exclude <path-or-glob>`: Exclude a file/folder/glob. Repeat the flag or pass comma-separated values.
- `-i, --include <path>`: Force-include specific file(s), even when excluded by ignore rules. Repeat or comma-separate values.
- `-F, --find-format <ext>`: Include only files with these extensions (example: `py` or `.py`). Repeat or comma-separate values.
- `-E, --exclude-format <ext>`: Exclude files with these extensions (example: `js` or `.js`). Repeat or comma-separate values.
- `--hidden`: Include hidden files (dotfiles). By default, hidden files are not included.
- `-p, --top-prompt <text>`: Prepend text at the very top of the output.
- `-r, --raw`: Print output to terminal (stdout) and do not create an output file.
- `-q, --quiet`: Hide default verbose completion logs.

By default, `prompter` prints a two-line completion summary to `stderr`:
- `<N> files found.`
- `<output-path> created`

## Rust Runtime Requirement

The npm package uses a Rust CLI under the hood and exposes a direct binary (`bin/prompter`).
On install, it builds the binary locally with Cargo (`cargo build --release`) and copies it to `bin/prompter`.

You need a Rust toolchain available in `PATH`:

```bash
rustup toolchain install stable
```

## Privacy

prompter runs entirely locally.
It does not send your code anywhere.

## License

MIT. See [LICENSE](LICENSE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for update guidelines (including testing, changelog, and versioning rules).
