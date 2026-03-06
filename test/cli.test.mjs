import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { mkdtemp, writeFile, access, readFile, mkdir } from "node:fs/promises";
import { constants } from "node:fs";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";

const cliPath = path.join(process.cwd(), "bin", "prompter");

test("cli raw mode writes to stdout, prepends top prompt, and respects excludes", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "prompter-test-"));

  await writeFile(path.join(root, "keep.txt"), "keep me", "utf8");
  await writeFile(path.join(root, "skip.txt"), "skip me", "utf8");

  const result = spawnSync(
    cliPath,
    [root, "--raw", "--top-prompt", "TOP HEADER", "--exclude", "skip.txt"],
    {
      encoding: "utf8",
      cwd: root,
    }
  );

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /^TOP HEADER\n\n# `.*` snapshot/);
  assert.doesNotMatch(result.stdout, /Files included:/);
  assert.doesNotMatch(result.stdout, /^---$/m);
  assert.match(result.stdout, /## keep\.txt/);
  assert.doesNotMatch(result.stdout, /## skip\.txt/);

  const defaultOutputPath = path.join(root, "prompter-output.md");
  await assert.rejects(access(defaultOutputPath, constants.F_OK), /ENOENT/);
});

test("cli defaults to current directory when no positional path is provided", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "prompter-test-"));
  await writeFile(path.join(root, "cwd-default.txt"), "yes", "utf8");

  const result = spawnSync(cliPath, ["--raw"], {
    encoding: "utf8",
    cwd: root,
  });

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /## cwd-default\.txt/);
});

test("cli excludes hidden files by default and includes them with --hidden", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "prompter-test-"));

  await writeFile(path.join(root, "visible.txt"), "visible", "utf8");
  await writeFile(path.join(root, ".env"), "SECRET=1", "utf8");

  const defaultResult = spawnSync(cliPath, [root, "--raw"], {
    encoding: "utf8",
    cwd: root,
  });

  assert.equal(defaultResult.status, 0, defaultResult.stderr);
  assert.match(defaultResult.stdout, /## visible\.txt/);
  assert.doesNotMatch(defaultResult.stdout, /## \.env/);

  const hiddenResult = spawnSync(cliPath, [root, "--raw", "--hidden"], {
    encoding: "utf8",
    cwd: root,
  });

  assert.equal(hiddenResult.status, 0, hiddenResult.stderr);
  assert.match(hiddenResult.stdout, /## \.env/);
});

test("cli default output file is overwritten without self-including previous snapshots", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "prompter-test-"));
  const outputPath = path.join(root, "prompter-output.md");

  await writeFile(path.join(root, "alpha.txt"), "alpha", "utf8");

  const firstRun = spawnSync(cliPath, [root], {
    encoding: "utf8",
    cwd: root,
  });
  assert.equal(firstRun.status, 0, firstRun.stderr);

  const firstOutput = await readFile(outputPath, "utf8");
  assert.match(firstOutput, /## alpha\.txt/);
  assert.doesNotMatch(firstOutput, /## prompter-output\.md/);

  await writeFile(path.join(root, "beta.txt"), "beta", "utf8");

  const secondRun = spawnSync(cliPath, [root], {
    encoding: "utf8",
    cwd: root,
  });
  assert.equal(secondRun.status, 0, secondRun.stderr);

  const secondOutput = await readFile(outputPath, "utf8");
  assert.match(secondOutput, /## beta\.txt/);
  assert.doesNotMatch(secondOutput, /## prompter-output\.md/);
});

test("cli exclude patterns support folders and globs", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "prompter-test-"));
  await mkdir(path.join(root, "src"), { recursive: true });
  await mkdir(path.join(root, "dist"), { recursive: true });

  await writeFile(path.join(root, "src", "cli.ts"), "x", "utf8");
  await writeFile(path.join(root, "src", "scan.ts"), "x", "utf8");
  await writeFile(path.join(root, "README.md"), "x", "utf8");
  await writeFile(path.join(root, "dist", "cli.js"), "x", "utf8");

  const folderExclude = spawnSync(
    cliPath,
    [root, "--raw", "--exclude", "README.md", "--exclude", "dist"],
    { encoding: "utf8", cwd: root }
  );

  assert.equal(folderExclude.status, 0, folderExclude.stderr);
  assert.match(folderExclude.stdout, /## src\/cli\.ts/);
  assert.doesNotMatch(folderExclude.stdout, /## README\.md/);
  assert.doesNotMatch(folderExclude.stdout, /## dist\/cli\.js/);

  const globExclude = spawnSync(
    cliPath,
    [root, "--raw", "--exclude", "src/*.ts"],
    { encoding: "utf8", cwd: root }
  );

  assert.equal(globExclude.status, 0, globExclude.stderr);
  assert.doesNotMatch(globExclude.stdout, /## src\/cli\.ts/);
  assert.doesNotMatch(globExclude.stdout, /## src\/scan\.ts/);
  assert.match(globExclude.stdout, /## README\.md/);
});

test("cli supports find-format and exclude-format filters", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "prompter-test-"));
  await writeFile(path.join(root, "main.py"), "print('x')", "utf8");
  await writeFile(path.join(root, "helper.js"), "console.log('x')", "utf8");
  await writeFile(path.join(root, "notes.md"), "hello", "utf8");

  const includeOnlyPy = spawnSync(
    cliPath,
    [root, "--raw", "--find-format", "py"],
    { encoding: "utf8", cwd: root }
  );

  assert.equal(includeOnlyPy.status, 0, includeOnlyPy.stderr);
  assert.match(includeOnlyPy.stdout, /## main\.py/);
  assert.doesNotMatch(includeOnlyPy.stdout, /## helper\.js/);
  assert.doesNotMatch(includeOnlyPy.stdout, /## notes\.md/);

  const excludeJs = spawnSync(
    cliPath,
    [root, "--raw", "--exclude-format", ".js"],
    { encoding: "utf8", cwd: root }
  );

  assert.equal(excludeJs.status, 0, excludeJs.stderr);
  assert.match(excludeJs.stdout, /## main\.py/);
  assert.match(excludeJs.stdout, /## notes\.md/);
  assert.doesNotMatch(excludeJs.stdout, /## helper\.js/);
});

test("cli include can force files excluded by default scanning rules", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "prompter-test-"));
  await writeFile(path.join(root, ".env.local"), "forced", "utf8");
  await writeFile(path.join(root, "visible.txt"), "visible", "utf8");

  const defaultResult = spawnSync(cliPath, [root, "--raw"], {
    encoding: "utf8",
    cwd: root,
  });
  assert.equal(defaultResult.status, 0, defaultResult.stderr);
  assert.match(defaultResult.stdout, /## visible\.txt/);
  assert.doesNotMatch(defaultResult.stdout, /## \.env\.local/);

  const includeForced = spawnSync(
    cliPath,
    [root, "--raw", "--include", ".env.local"],
    { encoding: "utf8", cwd: root }
  );
  assert.equal(includeForced.status, 0, includeForced.stderr);
  assert.match(includeForced.stdout, /## \.env\.local/);
});

test("cli positional file inputs include only explicit files", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "prompter-test-"));
  await mkdir(path.join(root, "ext"), { recursive: true });

  await writeFile(path.join(root, "main.py"), "print('main')", "utf8");
  await writeFile(path.join(root, "ext", "super.py"), "print('super')", "utf8");
  await writeFile(path.join(root, "skip.ts"), "console.log('skip')", "utf8");
  await writeFile(path.join(root, "super.ts"), "console.log('local')", "utf8");

  const result = spawnSync(
    cliPath,
    [path.join(root, "main.py"), path.join(root, "ext", "super.py"), "super.ts", "--raw"],
    {
      encoding: "utf8",
      cwd: root,
    }
  );

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /## .*main\.py/);
  assert.match(result.stdout, /## .*ext\/super\.py/);
  assert.match(result.stdout, /## super\.ts/);
  assert.doesNotMatch(result.stdout, /## skip\.ts/);
});

test("cli supports mixing multiple folders and files in positional inputs", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "prompter-test-"));
  await mkdir(path.join(root, "app"), { recursive: true });
  await mkdir(path.join(root, "lib"), { recursive: true });

  await writeFile(path.join(root, "app", "a.ts"), "export const a = 1;", "utf8");
  await writeFile(path.join(root, "lib", "b.py"), "print('b')", "utf8");
  await writeFile(path.join(root, "note.md"), "hello", "utf8");
  await writeFile(path.join(root, "other.txt"), "skip", "utf8");

  const result = spawnSync(cliPath, ["app", "lib", "note.md", "--raw"], {
    encoding: "utf8",
    cwd: root,
  });

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /## app\/a\.ts/);
  assert.match(result.stdout, /## lib\/b\.py/);
  assert.match(result.stdout, /## note\.md/);
  assert.doesNotMatch(result.stdout, /## other\.txt/);
});

test("cli verbose is enabled by default and hidden with --quiet", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "prompter-test-"));
  await writeFile(path.join(root, "one.txt"), "1", "utf8");

  const verboseResult = spawnSync(cliPath, [root, "--raw"], {
    encoding: "utf8",
    cwd: root,
  });
  assert.equal(verboseResult.status, 0, verboseResult.stderr);
  assert.match(verboseResult.stderr, /1 files found\nstdout created/);

  const quietResult = spawnSync(cliPath, [root, "--raw", "--quiet"], {
    encoding: "utf8",
    cwd: root,
  });
  assert.equal(quietResult.status, 0, quietResult.stderr);
  assert.equal(quietResult.stderr, "");
});
