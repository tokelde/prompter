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
  assert.match(result.stdout, /^TOP HEADER\n\n# Repository Snapshot/);
  assert.match(result.stdout, /## keep\.txt/);
  assert.doesNotMatch(result.stdout, /## skip\.txt/);

  const defaultOutputPath = path.join(root, "prompter-output.md");
  await assert.rejects(access(defaultOutputPath, constants.F_OK), /ENOENT/);
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
