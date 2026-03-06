use clap::{ArgAction, Parser};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "prompter",
    version,
    about = "Bundle your repository into a single Markdown document for LLM prompts."
)]
struct Cli {
    #[arg(value_name = "path-or-file")]
    inputs: Vec<String>,

    #[arg(short, long)]
    contains: Option<String>,

    #[arg(short = 'o', long)]
    out: Option<String>,

    #[arg(short, long = "exclude", value_name = "path-or-glob", action = ArgAction::Append)]
    exclude: Vec<String>,

    #[arg(short = 'i', long = "include", value_name = "path", action = ArgAction::Append)]
    include: Vec<String>,

    #[arg(short = 'F', long = "find-format", value_name = "ext", action = ArgAction::Append)]
    include_format: Vec<String>,

    #[arg(short = 'E', long = "exclude-format", value_name = "ext", action = ArgAction::Append)]
    exclude_format: Vec<String>,

    #[arg(long)]
    hidden: bool,

    #[arg(short = 'p', long = "top-prompt")]
    top_prompt: Option<String>,

    #[arg(short, long)]
    raw: bool,

    #[arg(long = "no-ignore", action = ArgAction::SetTrue)]
    no_ignore: bool,

    #[arg(short = 'q', long = "quiet", action = ArgAction::SetTrue)]
    quiet: bool,
}

#[derive(Clone)]
struct SnapshotFile {
    path: String,
    content: String,
}

struct ExcludeMatcher {
    globs: GlobSet,
    exact_files: Vec<String>,
    dir_prefixes: Vec<String>,
}

#[derive(Clone)]
struct InputFile {
    path: String,
    absolute: PathBuf,
}

#[derive(Clone)]
struct InputDirectory {
    path: String,
    absolute: PathBuf,
}

struct InputTarget {
    directories: Vec<InputDirectory>,
    files: Vec<InputFile>,
}

impl ExcludeMatcher {
    fn new(patterns: &[String]) -> Result<Self, String> {
        let mut builder = GlobSetBuilder::new();
        let mut exact_files = Vec::new();
        let mut dir_prefixes = Vec::new();

        for pattern in patterns {
            let normalized = normalize_pattern(pattern);
            if normalized.is_empty() {
                continue;
            }

            if has_glob(&normalized) {
                let glob = Glob::new(&normalized)
                    .map_err(|error| format!("Invalid exclude pattern '{normalized}': {error}"))?;
                builder.add(glob);
                continue;
            }

            let trimmed = normalized.trim_end_matches('/').to_string();
            if !trimmed.is_empty() {
                exact_files.push(trimmed.clone());
                dir_prefixes.push(format!("{trimmed}/"));
            }
        }

        let globs = builder
            .build()
            .map_err(|error| format!("Could not compile exclude patterns: {error}"))?;

        Ok(Self {
            globs,
            exact_files,
            dir_prefixes,
        })
    }

    fn is_excluded(&self, relative_path: &str) -> bool {
        if self.globs.is_match(relative_path) {
            return true;
        }

        if self.exact_files.iter().any(|pattern| pattern == relative_path) {
            return true;
        }

        self.dir_prefixes
            .iter()
            .any(|prefix| relative_path.starts_with(prefix))
    }
}

fn normalize_pattern(input: &str) -> String {
    let mut normalized = input.trim().replace('\\', "/");
    while normalized.starts_with("./") {
        normalized = normalized[2..].to_string();
    }
    while normalized.starts_with('/') {
        normalized = normalized[1..].to_string();
    }
    normalized
}

fn split_multi_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(|entry| entry.to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn normalize_display_path(input: &Path) -> String {
    let path = input.to_string_lossy().replace('\\', "/");
    if path.is_empty() {
        ".".to_string()
    } else {
        path
    }
}

fn normalize_extension(input: &str) -> String {
    let trimmed = input.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return trimmed;
    }
    if trimmed.starts_with('.') {
        trimmed
    } else {
        format!(".{trimmed}")
    }
}

fn file_extension(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy().to_ascii_lowercase()))
}

fn has_glob(input: &str) -> bool {
    input
        .chars()
        .any(|ch| matches!(ch, '*' | '?' | '[' | ']' | '{' | '}' | '!'))
}

fn to_posix(relative_path: &Path) -> String {
    let mut output = String::new();
    for (index, component) in relative_path.components().enumerate() {
        if index > 0 {
            output.push('/');
        }
        output.push_str(&component.as_os_str().to_string_lossy());
    }
    output
}

fn should_skip_node_modules(relative_path: &str) -> bool {
    relative_path == "node_modules"
        || relative_path.starts_with("node_modules/")
        || relative_path.contains("/node_modules/")
}

fn scan_files(root: &Path, respect_gitignore: bool, include_hidden: bool) -> Result<Vec<String>, String> {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(!include_hidden)
        .follow_links(false)
        .git_ignore(respect_gitignore)
        .git_global(respect_gitignore)
        .git_exclude(respect_gitignore)
        .parents(respect_gitignore)
        .ignore(false);

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry.map_err(|error| format!("Error scanning files: {error}"))?;
        if !entry
            .file_type()
            .map(|file_type| file_type.is_file())
            .unwrap_or(false)
        {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|error| format!("Could not compute relative path: {error}"))?;
        let relative_posix = to_posix(relative);

        if should_skip_node_modules(&relative_posix) {
            continue;
        }

        files.push(relative_posix);
    }

    files.sort();
    Ok(files)
}

fn load_snapshot_files_from_inputs(files: &[InputFile], contains: Option<&str>) -> Vec<SnapshotFile> {
    let mut snapshots = Vec::new();

    for input in files {
        let content = match fs::read_to_string(&input.absolute) {
            Ok(content) => content,
            Err(_) => continue,
        };

        if let Some(term) = contains {
            if !content.contains(term) {
                continue;
            }
        }

        snapshots.push(SnapshotFile {
            path: input.path.clone(),
            content,
        });
    }

    snapshots
}

fn language_by_extension() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        (".ts", "ts"),
        (".tsx", "tsx"),
        (".js", "js"),
        (".jsx", "jsx"),
        (".json", "json"),
        (".md", "md"),
        (".txt", "text"),
        (".html", "html"),
        (".css", "css"),
        (".scss", "scss"),
        (".yml", "yaml"),
        (".yaml", "yaml"),
        (".xml", "xml"),
        (".toml", "toml"),
        (".sh", "bash"),
        (".zsh", "bash"),
        (".bash", "bash"),
        (".py", "python"),
        (".rb", "ruby"),
        (".go", "go"),
        (".rs", "rust"),
        (".java", "java"),
        (".kt", "kotlin"),
        (".swift", "swift"),
        (".c", "c"),
        (".h", "c"),
        (".cpp", "cpp"),
        (".hpp", "cpp"),
        (".sql", "sql"),
    ])
}

fn detect_language(relative_path: &str, language_map: &HashMap<&str, &str>) -> String {
    let extension = Path::new(relative_path)
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy().to_ascii_lowercase()));

    extension
        .as_ref()
        .and_then(|extension| language_map.get(extension.as_str()))
        .copied()
        .unwrap_or("text")
        .to_string()
}

fn create_fence(content: &str) -> String {
    let mut longest = 0usize;
    let mut current = 0usize;

    for ch in content.chars() {
        if ch == '`' {
            current += 1;
            if current > longest {
                longest = current;
            }
        } else {
            current = 0;
        }
    }

    "`".repeat(std::cmp::max(3, longest + 1))
}

fn command_used() -> String {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        ".".to_string()
    } else {
        args.join(" ")
    }
}

fn format_snapshot(files: &[SnapshotFile], command: &str) -> String {
    let language_map = language_by_extension();
    let mut lines = vec![format!("# `{}` snapshot", command), String::new()];

    for snapshot in files {
        let language = detect_language(&snapshot.path, &language_map);
        let fence = create_fence(&snapshot.content);

        lines.push(format!("## {}", snapshot.path));
        lines.push(String::new());
        lines.push(format!("{}{}", fence, language));
        lines.push(snapshot.content.clone());
        lines.push(fence);
        lines.push(String::new());
    }

    lines.join("\n")
}

fn real_absolute_path(path: &Path) -> Result<PathBuf, String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    let real_parent = fs::canonicalize(parent)
        .map_err(|error| format!("Could not resolve output directory: {error}"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("Invalid output path: {}", path.display()))?;
    Ok(real_parent.join(file_name))
}

fn is_inside_root(root: &Path, absolute_path: &Path) -> Option<String> {
    let relative = absolute_path.strip_prefix(root).ok()?;
    let relative_path = to_posix(relative);
    if relative_path.is_empty() {
        None
    } else {
        Some(relative_path)
    }
}

fn resolve_include_path(root: &Path, include_value: &str) -> Result<String, String> {
    let normalized = normalize_pattern(include_value);
    if normalized.is_empty() {
        return Err("Include path cannot be empty".to_string());
    }

    let candidate = PathBuf::from(&normalized);
    let absolute = if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    };

    let metadata = fs::metadata(&absolute)
        .map_err(|error| format!("Could not read include path '{include_value}': {error}"))?;
    if !metadata.is_file() {
        return Err(format!("Include path '{include_value}' is not a file"));
    }

    is_inside_root(root, &absolute).ok_or_else(|| {
        format!(
            "Include path '{}' must be inside root '{}'",
            include_value,
            root.display()
        )
    })
}

fn resolve_input_file(cwd: &Path, value: &str) -> Result<InputFile, String> {
    let candidate = PathBuf::from(value);
    let absolute = if candidate.is_absolute() {
        candidate.clone()
    } else {
        cwd.join(&candidate)
    };
    let absolute = fs::canonicalize(&absolute)
        .map_err(|error| format!("Could not resolve input path '{}': {error}", value))?;
    let metadata = fs::metadata(&absolute)
        .map_err(|error| format!("Could not read input path '{}': {error}", value))?;
    if !metadata.is_file() {
        return Err(format!("Input path '{}' is not a file", value));
    }

    let mut display = if candidate.is_absolute() {
        normalize_display_path(&absolute)
    } else {
        normalize_pattern(value)
    };
    if display.is_empty() {
        display = ".".to_string();
    }

    Ok(InputFile {
        path: display,
        absolute,
    })
}

fn resolve_input_target(inputs: &[String]) -> Result<InputTarget, String> {
    let raw_inputs = if inputs.is_empty() {
        vec![".".to_string()]
    } else {
        inputs.to_vec()
    };

    let cwd = fs::canonicalize(Path::new("."))
        .map_err(|error| format!("Could not resolve current directory: {error}"))?;
    let mut files = Vec::new();
    let mut directories = Vec::new();

    for value in raw_inputs {
        let candidate = PathBuf::from(&value);
        let absolute = if candidate.is_absolute() {
            candidate.clone()
        } else {
            cwd.join(&candidate)
        };
        let absolute = fs::canonicalize(&absolute)
            .map_err(|error| format!("Could not resolve input path '{}': {error}", value))?;
        let metadata = fs::metadata(&absolute)
            .map_err(|error| format!("Could not read input path '{}': {error}", value))?;

        if metadata.is_dir() {
            let mut display = if candidate.is_absolute() {
                normalize_display_path(&absolute)
            } else {
                normalize_pattern(&value)
            };
            if display.is_empty() {
                display = ".".to_string();
            }
            directories.push(InputDirectory {
                path: display,
                absolute,
            });
            continue;
        }

        if metadata.is_file() {
            files.push(resolve_input_file(&cwd, &value)?);
            continue;
        }

        return Err(format!("Input path '{}' is neither a file nor directory", value));
    }

    directories.sort_by(|left, right| left.path.cmp(&right.path));
    directories.dedup_by(|left, right| left.absolute == right.absolute);

    files.sort_by(|left, right| left.path.cmp(&right.path));
    files.dedup_by(|left, right| left.absolute == right.absolute);

    Ok(InputTarget { directories, files })
}

fn print_verbose_summary(files_found: usize, created_target: &str) {
    eprintln!("{files_found} files found");
    eprintln!("{created_target} created");
}

fn with_prefix(prefix: &str, relative: &str) -> String {
    if prefix == "." {
        return format!("./{relative}");
    }
    format!("{}/{}", prefix.trim_end_matches('/'), relative)
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let target = resolve_input_target(&cli.inputs)?;

    let output_path = cli.out.unwrap_or_else(|| "prompter-output.md".to_string());
    let output_absolute = real_absolute_path(&PathBuf::from(&output_path))?;

    let exclude_patterns = split_multi_values(&cli.exclude);
    let include_patterns = split_multi_values(&cli.include);
    let include_formats: Vec<String> = split_multi_values(&cli.include_format)
        .into_iter()
        .map(|value| normalize_extension(&value))
        .filter(|value| !value.is_empty())
        .collect();
    let exclude_formats: Vec<String> = split_multi_values(&cli.exclude_format)
        .into_iter()
        .map(|value| normalize_extension(&value))
        .filter(|value| !value.is_empty())
        .collect();

    let matcher = ExcludeMatcher::new(&exclude_patterns)?;
    let has_multiple_sources = target.directories.len() + target.files.len() > 1;
    let mut resolved_inputs = target.files.clone();

    for directory in &target.directories {
        let mut directory_files = scan_files(&directory.absolute, !cli.no_ignore, cli.hidden)?;
        if let Some(relative_output) = is_inside_root(&directory.absolute, &output_absolute) {
            directory_files.retain(|path| path != &relative_output);
        }

        for relative in directory_files {
            if matcher.is_excluded(&relative) {
                continue;
            }
            if !include_formats.is_empty()
                && file_extension(&relative)
                    .map(|ext| include_formats.iter().any(|candidate| candidate == &ext))
                    .unwrap_or(false)
                    == false
            {
                continue;
            }
            if !exclude_formats.is_empty()
                && file_extension(&relative)
                    .map(|ext| exclude_formats.iter().any(|candidate| candidate == &ext))
                    .unwrap_or(false)
            {
                continue;
            }

            let path = if has_multiple_sources {
                with_prefix(&directory.path, &relative)
            } else {
                relative.clone()
            };
            resolved_inputs.push(InputFile {
                path,
                absolute: directory.absolute.join(relative),
            });
        }
    }

    if !include_patterns.is_empty() {
        if target.directories.len() != 1 || !target.files.is_empty() {
            return Err("--include requires exactly one directory positional input".to_string());
        }

        let root = &target.directories[0].absolute;
        for include_pattern in include_patterns {
            let relative = resolve_include_path(root, &include_pattern)?;
            let path = if has_multiple_sources {
                with_prefix(&target.directories[0].path, &relative)
            } else {
                relative.clone()
            };
            let absolute = root.join(relative);
            if !resolved_inputs.iter().any(|existing| existing.absolute == absolute) {
                resolved_inputs.push(InputFile { path, absolute });
            }
        }
    }

    resolved_inputs.sort_by(|left, right| left.path.cmp(&right.path));
    resolved_inputs.dedup_by(|left, right| left.absolute == right.absolute);

    let snapshots = load_snapshot_files_from_inputs(&resolved_inputs, cli.contains.as_deref());
    let markdown = format_snapshot(&snapshots, &command_used());
    let output = if let Some(top_prompt) = cli.top_prompt {
        format!("{top_prompt}\n\n{markdown}")
    } else {
        markdown
    };

    if cli.raw {
        print!("{output}");
        io::stdout()
            .flush()
            .map_err(|error| format!("Could not flush stdout: {error}"))?;
        if !cli.quiet {
            print_verbose_summary(snapshots.len(), "stdout");
        }
        return Ok(());
    }

    fs::write(&output_path, output).map_err(|error| format!("Could not write output file: {error}"))?;
    if !cli.quiet {
        print_verbose_summary(snapshots.len(), &output_path);
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
