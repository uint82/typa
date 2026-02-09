use anyhow::Result;
use config::{Config, File};
use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Theme {
    pub bg: String,      // background
    pub main: String,    // brand color (timer, active highlights)
    pub caret: String,   // cursor block color
    pub text: String,    // correct text
    pub sub: String,     // untyped / future text / unactive
    #[serde(alias = "subAlt")]
    pub sub_alt: String, // subtle UI elements (footer, borders)
    pub error: String,   // incorrect / extra text
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: "#2c2e34".to_string(),
            main: "#e2b714".to_string(),
            caret: "#e2b714".to_string(),
            text: "#d1d0c5".to_string(),
            sub: "#646669".to_string(),
            sub_alt: "#45474d".to_string(),
            error: "#ca4754".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub theme: Theme,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let defaults = Theme::default();

        let mut builder = Config::builder()
            .set_default("theme.bg", defaults.bg)?
            .set_default("theme.main", defaults.main)?
            .set_default("theme.caret", defaults.caret)?
            .set_default("theme.text", defaults.text)?
            .set_default("theme.sub", defaults.sub)?
            .set_default("theme.subAlt", defaults.sub_alt)?
            .set_default("theme.error", defaults.error)?;

        if let Some(proj_dirs) = ProjectDirs::from("", "", "typa") {
            let config_dir = proj_dirs.config_dir();
            let config_path = config_dir.join("config.toml");

            if config_path.exists() {
                builder = builder.add_source(File::from(config_path));
            }
        }

        let cfg = builder.build()?;

        // map "subAlt"  to "sub_alt"
        let app_config: AppConfig = cfg.try_deserialize()?;

        Ok(app_config)
    }
}
