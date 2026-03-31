use std::fs;

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
        let lsp_settings = LspSettings::for_worktree("sprocket", worktree)?;

        // 1. User-configured binary path — no version management.
        if let Some(settings) = &lsp_settings.settings
            && let Some(binary_path) = settings.get("binaryPath").and_then(|v| v.as_str())
        {
            return Ok(binary_path.to_string());
        }

        // 2. System PATH — no version management.
        if let Some(path) = worktree.which("sprocket") {
            return Ok(path);
        }

        // 3. Zed-managed binary — install or update as needed.
        let check_for_updates = lsp_settings
            .settings
            .as_ref()
            .and_then(|s| s.get("checkForUpdates"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let installed = self.installed_binary_path();

        if !check_for_updates {
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
                     and set the `binary.path` option in your Zed settings"
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
        let lsp_settings = LspSettings::for_worktree("sprocket", worktree)?;

        let mut args = vec!["analyzer".to_string(), "--stdio".to_string()];

        if let Some(settings) = &lsp_settings.settings
            && let Some(server_settings) = settings.get("server")
        {
            if let Some(lint_options) = server_settings.get("lint") {
                if lint_options.get("enabled").and_then(|v| v.as_bool()) == Some(true) {
                    args.push("--lint".to_string());
                }
            }

            match server_settings
                .get("logLevel")
                .and_then(|v| v.as_str())
                .map(|level| level.to_lowercase())
                .as_deref()
            {
                Some("trace") => args.push("-vvv".to_string()),
                Some("debug") => args.push("-vv".to_string()),
                Some("info") => args.push("-v".to_string()),
                Some("warn") => {}
                Some("error") | None => args.push("-q".to_string()),
                _ => {}
            }
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
        let lsp_settings = LspSettings::for_worktree("sprocket", worktree)?;

        if let Some(settings) = lsp_settings.settings
            && let Some(server_settings) = settings.get("server")
        {
            return Ok(Some(serde_json::json!({
                "sprocket.server": server_settings
            })));
        }

        Ok(None)
    }
}

zed::register_extension!(SprocketExtension);
