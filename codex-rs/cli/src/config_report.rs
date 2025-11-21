use codex_common::CliConfigOverrides;
use codex_core::config::Config;
use codex_core::config::ConfigOverrides;
use codex_core::config::find_codex_home;
use codex_core::config::load_raw_config_with_cli_overrides;
use codex_core::model_provider_info::LMSTUDIO_OSS_PROVIDER_ID;
use codex_core::model_provider_info::OLLAMA_OSS_PROVIDER_ID;
use owo_colors::OwoColorize;
use std::path::PathBuf;
use toml::Value as TomlValue;

pub async fn print_config(
    cmd_overrides: CliConfigOverrides,
    profile: Option<String>,
) -> anyhow::Result<()> {
    let cli_overrides = cmd_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let toml_overrides: Vec<(String, TomlValue)> = cli_overrides
        .into_iter()
        .map(|(k, v)| (k, v.into()))
        .collect();

    let config = Config::load_with_cli_overrides(
        toml_overrides.clone(),
        ConfigOverrides {
            config_profile: profile.clone(),
            ..Default::default()
        },
    )
    .await?;

    let codex_home = find_codex_home()?;
    let raw_config = load_raw_config_with_cli_overrides(&codex_home, toml_overrides).await?;
    let active_profile = config.active_profile.clone();

    println!(
        "{}",
        format_config_report(&config, &raw_config, &cmd_overrides, active_profile)
    );

    Ok(())
}

fn format_config_report(
    config: &Config,
    raw_config: &TomlValue,
    cli_overrides: &CliConfigOverrides,
    active_profile: Option<String>,
) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push("Codex configuration".bold().to_string());
    lines.push(String::new());

    let profile_label = active_profile
        .as_ref()
        .map(|p| format!("{p} (active)"))
        .unwrap_or_else(|| "none".to_string());
    lines.push(format!("Profile: {profile_label}"));

    let provider_source = value_source(
        raw_config,
        cli_overrides,
        &active_profile,
        &["model_provider"],
    );
    lines.push(format!(
        "Model provider: {} [{}]",
        config.model_provider_id, provider_source,
    ));

    let base_url_source = value_source(
        raw_config,
        cli_overrides,
        &active_profile,
        &[
            "model_providers",
            config.model_provider_id.as_str(),
            "base_url",
        ],
    );
    let base_url = config
        .model_provider
        .base_url
        .as_deref()
        .unwrap_or("default (built-in)");
    lines.push(format!("Provider base URL: {base_url} [{base_url_source}]"));

    let model_source = value_source(raw_config, cli_overrides, &active_profile, &["model"]);
    lines.push(format!("Model: {} [{model_source}]", config.model));

    let review_model_source = value_source(
        raw_config,
        cli_overrides,
        &active_profile,
        &["review_model"],
    );
    lines.push(format!(
        "Review model: {} [{review_model_source}]",
        config.review_model
    ));

    let oss_provider_source = value_source(
        raw_config,
        cli_overrides,
        &active_profile,
        &["oss_provider"],
    );
    lines.push(format!(
        "Preferred OSS provider: {} [{oss_provider_source}]",
        resolved_oss_provider_label(raw_config)
    ));

    let approval_source = value_source(
        raw_config,
        cli_overrides,
        &active_profile,
        &["approval_policy"],
    );
    lines.push(format!(
        "Approval policy: {:?} [{approval_source}]",
        config.approval_policy
    ));

    let sandbox_source = value_source(
        raw_config,
        cli_overrides,
        &active_profile,
        &["sandbox_mode"],
    );
    lines.push(format!(
        "Sandbox policy: {:?} [{}]",
        config.sandbox_policy, sandbox_source
    ));

    let cwd_source = value_source(raw_config, cli_overrides, &active_profile, &["cwd"]);
    lines.push(format!(
        "Working directory: {} [{}]",
        format_path(&config.cwd),
        cwd_source
    ));

    let notification_source = value_source(raw_config, cli_overrides, &active_profile, &["notify"]);
    let notify = config
        .notify
        .as_ref()
        .map(|cmd| cmd.join(" "))
        .unwrap_or_else(|| "none".to_string());
    lines.push(format!("Notifier: {notify} [{notification_source}]"));

    let reasoning_source = value_source(
        raw_config,
        cli_overrides,
        &active_profile,
        &["show_raw_agent_reasoning"],
    );
    lines.push(format!(
        "Show raw reasoning: {} [{reasoning_source}]",
        config.show_raw_agent_reasoning
    ));

    lines.join("\n")
}

fn resolved_oss_provider_label(raw_config: &TomlValue) -> String {
    if has_path(raw_config, &["oss_provider"]) {
        raw_config
            .get("oss_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("(unknown)")
            .to_string()
    } else {
        format!(
            "auto (checks {} then {})",
            LMSTUDIO_OSS_PROVIDER_ID,
            OLLAMA_OSS_PROVIDER_ID
        )
    }
}

fn value_source(
    raw_config: &TomlValue,
    cli_overrides: &CliConfigOverrides,
    active_profile: &Option<String>,
    path: &[&str],
) -> String {
    let cli_has_path = cli_overrides
        .raw_overrides
        .iter()
        .any(|override_key| override_key.starts_with(&path.join(".")));

    if cli_has_path {
        return "cli override".to_string();
    }

    if let Some(profile) = active_profile {
        let mut full = vec!["profiles", profile.as_str()];
        full.extend_from_slice(path);
        if has_path(raw_config, &full) {
            return format!("profile `{profile}`");
        }
    }

    if has_path(raw_config, path) {
        "config.toml".to_string()
    } else {
        "default".to_string()
    }
}

fn has_path(value: &TomlValue, path: &[&str]) -> bool {
    let mut current = value;
    for segment in path {
        match current {
            TomlValue::Table(tbl) => {
                if let Some(next) = tbl.get(*segment) {
                    current = next;
                } else {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

fn format_path(path: &PathBuf) -> String {
    path.to_string_lossy().to_string()
}
