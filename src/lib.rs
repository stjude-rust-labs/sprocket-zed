use std::fmt::Formatter;
use std::fs;

use serde::{Deserialize, Deserializer, Serialize};
use zed::LanguageServerId;
use zed::Result;
use zed::settings::LspSettings;
use zed_extension_api as zed;
use zed_extension_api::serde_json;

struct SprocketExtension {
    cached_binary_path: Option<String>,
}

impl SprocketExtension {
    /// Finds an existing Zed-managed Sprocket installation on disk by scanning
    /// for `sprocket-*` directories containing a `sprocket` binary.
    fn installed_binary_path(&self) -> Option<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).is_ok_and(|stat| stat.is_file()) {
                return Some(path.clone());
            }
        }

        let entries = fs::read_dir(".").ok()?;

        entries.flatten().find_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;

            if !name.starts_with("sprocket-") {
                return None;
            }

            let path = format!("{name}/sprocket");

            fs::metadata(&path)
                .is_ok_and(|stat| stat.is_file())
                .then_some(path)
        })
    }

    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        let settings = Settings::load(&worktree)?;

        // 1. User-configured binary path — no version management.
        if let Some(binary_path) = settings.binary_path {
            return Ok(binary_path.to_string());
        }

        // 2. System PATH — no version management.
        if let Some(path) = worktree.which("sprocket") {
            return Ok(path);
        }

        // 3. Zed-managed binary — install or update as needed.
        let installed = self.installed_binary_path();

        if !settings.check_for_updates {
            if let Some(path) = installed {
                self.cached_binary_path = Some(path.clone());
                return Ok(path);
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            "stjude-rust-labs/sprocket",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();

        let arch = match arch {
            zed::Architecture::Aarch64 => "aarch64",
            zed::Architecture::X8664 => "x86_64",
            zed::Architecture::X86 => {
                return Err("Sprocket does not provide prebuilt 32-bit x86 binaries; \
                     please build from source (https://github.com/stjude-rust-labs/sprocket) \
                     and set the `binaryPath` option in your Zed settings"
                    .into());
            }
        };

        let (os, ext) = match platform {
            zed::Os::Mac => ("apple-darwin", "tar.gz"),
            zed::Os::Linux => ("unknown-linux-gnu", "tar.gz"),
            zed::Os::Windows => ("pc-windows-msvc", "zip"),
        };

        let version_dir = format!("sprocket-{}", release.version);
        let binary_path = format!("{version_dir}/sprocket");

        // Already up to date.
        if installed.as_deref() == Some(binary_path.as_str()) {
            self.cached_binary_path = Some(binary_path.clone());
            return Ok(binary_path);
        }

        let asset_name = format!(
            "sprocket-{version}-{arch}-{os}.{ext}",
            version = release.version,
        );

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no Sprocket release available for {asset_name}"))?;

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        zed::download_file(
            &asset.download_url,
            &version_dir,
            if ext == "zip" {
                zed::DownloadedFileType::Zip
            } else {
                zed::DownloadedFileType::GzipTar
            },
        )
        .map_err(|e| format!("failed to download Sprocket: {e}"))?;

        zed::make_file_executable(&binary_path)?;

        // Clean up old version directories.
        let entries =
            fs::read_dir(".").map_err(|e| format!("failed to list working directory: {e}"))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;

            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };

            if name != version_dir && name.starts_with("sprocket-") {
                fs::remove_dir_all(entry.path()).ok();
            }
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Settings {
    binary_path: Option<String>,
    check_for_updates: bool,
    server: ServerSettings,
}

impl Settings {
    fn load(worktree: &zed_extension_api::Worktree) -> Result<Self> {
        let lsp_settings = LspSettings::for_worktree("sprocket", worktree)?;

        Ok(lsp_settings
            .settings
            .map(|lsp_settings| {
                serde_json::from_value::<Settings>(lsp_settings)
                    .map_err(|e| format!("failed to parse settings: {e}"))
            })
            .transpose()?
            .unwrap_or_default())
    }
}

#[derive(Serialize, Default)]
#[serde(rename_all = "lowercase")]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    #[default]
    Error,
}

impl LogLevel {
    const VARIANTS: &[&str] = &["trace", "debug", "info", "warn", "error"];
}

impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LogLevelVisitor;

        impl<'de> serde::de::Visitor<'de> for LogLevelVisitor {
            type Value = LogLevel;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "a log level string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v.to_lowercase().as_str() {
                    "trace" => Ok(LogLevel::Trace),
                    "debug" => Ok(LogLevel::Debug),
                    "info" => Ok(LogLevel::Info),
                    "warn" => Ok(LogLevel::Warn),
                    "error" => Ok(LogLevel::Error),
                    _ => Err(serde::de::Error::unknown_variant(v, LogLevel::VARIANTS)),
                }
            }
        }

        deserializer.deserialize_str(LogLevelVisitor)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ServerSettings {
    log_level: LogLevel,
    lint: LintSettings,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Error,
            lint: LintSettings::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct LintSettings {
    enabled: bool,
}

impl zed::Extension for SprocketExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary_path = self.language_server_binary_path(language_server_id, worktree)?;
        let settings = Settings::load(&worktree)?;

        let mut args = vec!["analyzer".to_string(), "--stdio".to_string()];

        if settings.server.lint.enabled {
            args.push("--lint".to_string());
        }

        match settings.server.log_level {
            LogLevel::Trace => args.push("-vvv".to_string()),
            LogLevel::Debug => args.push("-vv".to_string()),
            LogLevel::Info => args.push("-v".to_string()),
            LogLevel::Warn => {}
            LogLevel::Error => args.push("-q".to_string()),
        }

        Ok(zed::Command {
            command: binary_path,
            args,
            env: Default::default(),
        })
    }

    fn language_server_workspace_configuration(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed_extension_api::Worktree,
    ) -> Result<Option<zed_extension_api::serde_json::Value>> {
        let settings = Settings::load(&worktree)?;

        return Ok(Some(serde_json::json!({
            "sprocket.server": settings.server
        })));
    }
}

zed::register_extension!(SprocketExtension);
