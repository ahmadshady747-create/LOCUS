use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Default directories that should always be ignored during AST and symbol indexing
pub const DEFAULT_IGNORED_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    ".git",
    ".venv",
    "venv",
    "__pycache__",
    ".locus",
    ".cache",
    ".next",
    ".nuxt",
    ".output",
    ".turbo",
    ".idea",
    ".vscode",
    "coverage",
];

/// Known binary and asset file extensions that should not be indexed for AST/code symbols
pub const BINARY_EXTENSIONS: &[&str] = &[
    // Images
    "png", "jpg", "jpeg", "gif", "ico", "webp", "bmp", "tiff", "psd", "ai",
    // Audio / Video
    "mp3", "mp4", "wav", "flac", "ogg", "mov", "avi", "mkv", "webm",
    // Documents / Fonts
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "woff", "woff2", "ttf", "eot", "otf",
    // Archives & Compressed
    "zip", "tar", "gz", "7z", "rar", "bz2", "xz", "iso", "dmg",
    // Compiled & Executable
    "exe", "dll", "so", "dylib", "bin", "o", "obj", "a", "lib", "wasm", "class", "jar", "war",
    "pyc", "pyo", "pyd",
    // Databases & Large Datasets
    "db", "sqlite", "sqlite3", "parquet", "feather", "arrow", "h5", "hdf5",
];

/// Intelligent Ignore Engine combining default system ignores, binary heuristics, and .gitignore rules
pub struct IgnoreEngine {
    root_path: Option<PathBuf>,
    default_dirs: HashSet<String>,
    binary_extensions: HashSet<String>,
    gitignore: Option<Gitignore>,
    custom_glob_set: Option<GlobSet>,
    custom_patterns: Vec<String>,
}

impl IgnoreEngine {
    /// Creates an IgnoreEngine with standard default ignored folders and binary filters
    pub fn new() -> Self {
        let mut default_dirs = HashSet::new();
        for dir in DEFAULT_IGNORED_DIRS {
            default_dirs.insert(dir.to_lowercase());
        }

        let mut binary_extensions = HashSet::new();
        for ext in BINARY_EXTENSIONS {
            binary_extensions.insert(ext.to_lowercase());
        }

        Self {
            root_path: None,
            default_dirs,
            binary_extensions,
            gitignore: None,
            custom_glob_set: None,
            custom_patterns: Vec::new(),
        }
    }

    /// Initializes an IgnoreEngine configured for a specific workspace root,
    /// automatically discovering and loading `.gitignore` rules from that root.
    pub fn from_workspace_root<P: AsRef<Path>>(root: P) -> Self {
        let mut engine = Self::new();
        let root_path = root.as_ref().to_path_buf();
        engine.load_gitignore(&root_path);
        engine.root_path = Some(root_path);
        engine
    }

    /// Loads `.gitignore` rules from the specified workspace directory
    pub fn load_gitignore(&mut self, root: &Path) {
        let gitignore_path = root.join(".gitignore");
        if gitignore_path.exists() {
            let mut builder = GitignoreBuilder::new(root);
            if let Some(err) = builder.add(&gitignore_path) {
                debug!("Warning loading .gitignore at {}: {:?}", gitignore_path.display(), err);
            }
            match builder.build() {
                Ok(gi) => {
                    info!("Loaded .gitignore rules from {}", gitignore_path.display());
                    self.gitignore = Some(gi);
                }
                Err(e) => {
                    debug!("Failed to compile .gitignore rules: {:?}", e);
                }
            }
        }
    }

    /// Adds custom ignore glob patterns (e.g. `*.min.js`, `temp/**`)
    pub fn add_custom_patterns(&mut self, patterns: &[String]) {
        self.custom_patterns.extend_from_slice(patterns);
        let mut builder = GlobSetBuilder::new();
        for pat in &self.custom_patterns {
            if let Ok(glob) = Glob::new(pat) {
                builder.add(glob);
            }
        }
        self.custom_glob_set = builder.build().ok();
    }

    /// Determines whether a given path should be ignored during indexing
    pub fn should_ignore(&self, path: &Path, is_dir: bool) -> bool {
        // 1. Check default directory names in any component of the path
        for component in path.components() {
            if let Some(comp_str) = component.as_os_str().to_str() {
                if self.default_dirs.contains(&comp_str.to_lowercase()) {
                    return true;
                }
            }
        }

        // 2. If it's a file, check if it has a binary/asset extension
        if !is_dir && self.is_binary_extension(path) {
            return true;
        }

        // 3. Check .gitignore rules if present
        if let Some(ref gitignore) = self.gitignore {
            let relative_path = if let Some(ref root) = self.root_path {
                path.strip_prefix(root).unwrap_or(path)
            } else {
                path
            };

            let matched = gitignore.matched_path_or_any_parents(relative_path, is_dir);
            if matched.is_ignore() {
                return true;
            }
        }

        // 4. Check custom glob set
        if let Some(ref glob_set) = self.custom_glob_set {
            if glob_set.is_match(path) {
                return true;
            }
        }

        false
    }

    /// String-based ignore check (normalizing path separators)
    pub fn should_ignore_str(&self, path_str: &str) -> bool {
        let normalized = path_str.replace('\\', "/");
        let path = Path::new(&normalized);
        let is_dir = normalized.ends_with('/');
        self.should_ignore(path, is_dir)
    }

    /// Checks if a file path has a known binary or non-source asset extension
    pub fn is_binary_extension(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            self.binary_extensions.contains(&ext.to_lowercase())
        } else {
            false
        }
    }

    /// Fast heuristic check to detect if file content contains binary / null bytes
    /// Inspects the first 8 KB for zero (`\0`) bytes
    pub fn is_binary_content(bytes: &[u8]) -> bool {
        let sample_len = bytes.len().min(8192);
        let sample = &bytes[..sample_len];
        sample.contains(&0)
    }

    /// Filters a list of paths, returning only those that should be indexed
    pub fn filter_paths<'a, I>(&self, paths: I) -> Vec<&'a Path>
    where
        I: IntoIterator<Item = &'a Path>,
    {
        paths
            .into_iter()
            .filter(|p| !self.should_ignore(p, p.is_dir()))
            .collect()
    }
}

impl Default for IgnoreEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn test_default_ignored_folders() {
        let engine = IgnoreEngine::new();

        assert!(engine.should_ignore_str("node_modules/react/index.js"));
        assert!(engine.should_ignore_str("target/debug/build/locus.exe"));
        assert!(engine.should_ignore_str("dist/assets/index.js"));
        assert!(engine.should_ignore_str("build/output.js"));
        assert!(engine.should_ignore_str(".git/HEAD"));
        assert!(engine.should_ignore_str(".venv/lib/python3.11/site-packages"));
        assert!(engine.should_ignore_str("src/__pycache__/module.cpython-311.pyc"));
        assert!(engine.should_ignore_str(".locus/snapshots/123.bak"));

        // Valid source files should NOT be ignored
        assert!(!engine.should_ignore_str("src/main.rs"));
        assert!(!engine.should_ignore_str("crates/locus-core/src/lib.rs"));
        assert!(!engine.should_ignore_str("frontend/components/Chat.tsx"));
    }

    #[test]
    fn test_binary_extension_rejection() {
        let engine = IgnoreEngine::new();

        assert!(engine.is_binary_extension(Path::new("image.png")));
        assert!(engine.is_binary_extension(Path::new("photo.JPG")));
        assert!(engine.is_binary_extension(Path::new("document.pdf")));
        assert!(engine.is_binary_extension(Path::new("archive.tar.gz")));
        assert!(engine.is_binary_extension(Path::new("native.dll")));
        assert!(engine.is_binary_extension(Path::new("compiled.wasm")));

        assert!(!engine.is_binary_extension(Path::new("main.rs")));
        assert!(!engine.is_binary_extension(Path::new("app.tsx")));
        assert!(!engine.is_binary_extension(Path::new("script.py")));
    }

    #[test]
    fn test_binary_content_detection() {
        let text_bytes = b"pub fn hello_world() -> &'static str {\n    \"Hello LOCUS\"\n}\n";
        assert!(!IgnoreEngine::is_binary_content(text_bytes));

        let binary_bytes = b"\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x3e\x00";
        assert!(IgnoreEngine::is_binary_content(binary_bytes));
    }

    #[test]
    fn test_gitignore_rule_integration() {
        let temp_dir = std::env::temp_dir().join(format!("locus_test_ignore_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let gitignore_path = temp_dir.join(".gitignore");
        let mut f = File::create(&gitignore_path).unwrap();
        writeln!(f, "*.secret").unwrap();
        writeln!(f, "temp_data/").unwrap();
        writeln!(f, "!important.secret").unwrap();
        drop(f);

        let engine = IgnoreEngine::from_workspace_root(&temp_dir);

        let secret_file = temp_dir.join("credentials.secret");
        let temp_folder_file = temp_dir.join("temp_data").join("cache.json");
        let normal_file = temp_dir.join("src").join("main.rs");

        assert!(engine.should_ignore(&secret_file, false));
        assert!(engine.should_ignore(&temp_folder_file, false));
        assert!(!engine.should_ignore(&normal_file, false));

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
