use zed_extension_api::{
    self as zed,
    serde_json::{self, json, Value},
};

struct SuperColliderExtension;

fn dev_launcher_candidate(worktree: &zed::Worktree) -> Option<String> {
    // For development: return local release build if we're in the extension's source directory
    if worktree.read_text_file("Cargo.toml").is_ok() {
        let root = worktree.root_path();
        let path = format!("{}/server/launcher/target/release/sc_launcher", root);
        eprintln!("[supercollider] dev mode: using local launcher at {}", path);
        Some(path)
    } else {
        None
    }
}

fn is_supercollider_server(id: &zed::LanguageServerId) -> bool {
    id.as_ref().eq_ignore_ascii_case("supercollider")
}

fn default_workspace_settings() -> Value {
    json!({
        "supercollider": {
            "languageServerLogLevel": "error",
            "sclang": {
                "evaluateResultPrefix": "> ",
                "guestEvaluateResultPrefix": "[%|> ",
                "postEvaluateResults": "true",
                "improvedErrorReports": "false"
            }
        }
    })
}

fn merge_settings(base: &mut Value, overrides: &Value) {
    match (base, overrides) {
        (Value::Object(base_map), Value::Object(override_map)) => {
            for (key, value) in override_map {
                match base_map.get_mut(key) {
                    Some(base_value) => merge_settings(base_value, value),
                    None => {
                        base_map.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (base_slot, override_value) => {
            *base_slot = override_value.clone();
        }
    }
}

impl zed::Extension for SuperColliderExtension {
    fn new() -> Self {
        eprintln!("[supercollider] extension initialized");
        SuperColliderExtension
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        eprintln!(
            "[supercollider] language_server_command requested for id={}",
            language_server_id
        );
        // Accept either "supercollider" or "SuperCollider" defensively.
        if !is_supercollider_server(language_server_id) {
            return Err(format!(
                "unsupported language server id: {}",
                language_server_id
            ));
        }

        // Allow users to configure the launcher path/args/env via LSP settings.
        let lsp_settings =
            zed::settings::LspSettings::for_worktree("supercollider", worktree).unwrap_or_default();

        // Resolve command path: prefer settings.binary.path, otherwise try PATH for `sc_launcher`.
        let mut cmd_path = lsp_settings
            .binary
            .as_ref()
            .and_then(|b| b.path.clone())
            .or_else(|| worktree.which("sc_launcher"))
            .or_else(|| dev_launcher_candidate(worktree));

        if cmd_path.is_none() {
            eprintln!("[supercollider] no launcher found via settings or PATH");
            return Err("supercollider LSP launcher not found; set lsp.binary.path or add sc_launcher to PATH".into());
        }

        // Arguments and env from settings if provided.
        let mut args: Vec<String> = lsp_settings
            .binary
            .as_ref()
            .and_then(|b| b.arguments.clone())
            .unwrap_or_default();
        // Default to LSP mode if no args provided to reduce setup friction.
        if args.is_empty() {
            args = vec!["--mode".into(), "lsp".into()];
        }

        // Start with the worktree shell environment and apply any overrides from settings.
        let mut env: zed::EnvVars = worktree.shell_env();
        if let Some(bin) = lsp_settings.binary.as_ref() {
            if let Some(custom) = &bin.env {
                for (k, v) in custom.iter() {
                    env.push((k.clone(), v.clone()));
                }
            }
        }

        let cmd = zed::Command {
            command: cmd_path.take().unwrap(),
            args,
            env,
        };
        eprintln!(
            "[supercollider] launching LSP: {} {:?}",
            cmd.command, cmd.args
        );
        Ok(cmd)
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<Option<serde_json::Value>> {
        if !is_supercollider_server(language_server_id) {
            return Ok(None);
        }

        let lsp_settings =
            zed::settings::LspSettings::for_worktree("supercollider", worktree).unwrap_or_default();
        Ok(lsp_settings.initialization_options)
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<Option<serde_json::Value>> {
        if !is_supercollider_server(language_server_id) {
            return Ok(None);
        }

        let lsp_settings =
            zed::settings::LspSettings::for_worktree("supercollider", worktree).unwrap_or_default();
        let mut config = default_workspace_settings();

        if let Some(user_settings) = lsp_settings.settings {
            merge_settings(&mut config, &user_settings);
        }

        Ok(Some(config))
    }

    fn run_slash_command(
        &self,
        command: zed::SlashCommand,
        _args: Vec<String>,
        worktree: Option<&zed::Worktree>,
    ) -> zed::Result<zed::SlashCommandOutput> {
        if command.name != "supercollider-check-setup" {
            return Err("unknown slash command".into());
        }
        let Some(worktree) = worktree else {
            return Err("no worktree".into());
        };

        // Read launcher settings from LSP config for consistency with LSP startup.
        let lsp_settings =
            zed::settings::LspSettings::for_worktree("supercollider", worktree).unwrap_or_default();

        let mut cmd = if let Some(path) = lsp_settings.binary.as_ref().and_then(|b| b.path.clone())
        {
            zed::process::Command::new(path)
        } else if let Some(path) = worktree.which("sc_launcher") {
            zed::process::Command::new(path)
        } else if let Some(path) = dev_launcher_candidate(worktree) {
            zed::process::Command::new(path)
        } else {
            return Err(
                "supercollider launcher not found; set lsp.binary.path or add sc_launcher to PATH"
                    .into(),
            );
        };

        if let Some(bin) = lsp_settings.binary.as_ref() {
            if let Some(args) = &bin.arguments {
                cmd = cmd.args(args.iter().cloned());
            }
            if let Some(env) = &bin.env {
                cmd = cmd.envs(env.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }

        match cmd.output() {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let ok = out.status.unwrap_or(-1) == 0;
                let mut text = String::new();
                text.push_str("SuperCollider: Check Setup\n\n");
                text.push_str(&format!("status: {}\n", if ok { "ok" } else { "error" }));
                if !stdout.trim().is_empty() {
                    text.push_str("\nstdout:\n");
                    text.push_str(stdout.trim());
                    text.push('\n');
                }
                if !stderr.trim().is_empty() {
                    text.push_str("\nstderr:\n");
                    text.push_str(stderr.trim());
                    text.push('\n');
                }
                Ok(zed::SlashCommandOutput {
                    text,
                    sections: vec![],
                })
            }
            Err(e) => Err(format!("failed to run launcher: {e}")),
        }
    }
}

zed::register_extension!(SuperColliderExtension);
