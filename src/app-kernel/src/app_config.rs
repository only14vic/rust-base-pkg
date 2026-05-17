use {
    crate::prelude::App,
    alloc::{boxed::Box, format, string::String, vec::Vec},
    app_base::prelude::*,
    core::fmt::Display
};

#[derive(DebugExt, ExtendFromIter)]
pub struct AppConfig {
    pub name: Box<str>,
    pub version: Box<str>,
    pub no_std: bool,
    pub config_file: Box<str>,
    pub env_file: Option<Box<str>>,
    pub pid_file: Box<str>
}

impl AppConfig {
    pub const OPT_VERSION: &str = "V:version";
    pub const OPT_HELP: &str = "h:help";
    pub const OPT_VERBOSE: &str = "v:verbose";
    pub const OPT_DEBUG: &str = "D:debug";
    pub const OPT_FORCE: &str = "f:force";
    pub const OPT_ENV_FILE: &str = "E:env-file";
    pub const OPT_CONFIG_FILE: &str = "C:config-file";
    pub const OPT_PID_FILE: &str = "P:pid-file";
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: env!("APP_NAME").into(),
            version: concat!("v", env!("APP_VERSION"), " (", env!("BUILD_TIME"), ")")
                .into(),
            no_std: cfg!(not(feature = "std")),
            config_file: Env::get("CONFIG_FILE")
                .as_deref()
                .filter(|s| s.is_empty() == false)
                .or(option_env!("APP_CONFIG_FILE"))
                .unwrap_or("app.ini")
                .into(),
            env_file: Env::get("ENV_FILE")
                .as_deref()
                .filter(|s| s.is_empty() == false)
                .or(option_env!("APP_ENV_FILE"))
                .map(|s| s.into()),
            pid_file: Env::get("PID_FILE")
                .as_deref()
                .filter(|s| s.is_empty() == false)
                .unwrap_or(concat!(env!("APP_NAME"), ".pid"))
                .into()
        }
    }
}

impl IterConfig for AppConfig {
    fn iter_config(&self) -> Vec<(&'static str, String)> {
        [
            // app
            ("app.name", &self.name as &dyn Display),
            ("app.version", &self.version),
            ("app.config_file", &self.config_file),
            (
                "app.env_file",
                &self.env_file.as_ref().map(Box::as_ref).unwrap_or_default()
            ),
            ("app.pid_file", &self.pid_file),
            ("app.no_std", &self.no_std),
            ("app.profile", &env!("BUILD_PROFILE")),
            ("app.mods_stack_limit", &App::MODS_STACK_LIMIT),
            // env
            ("env.env", &Env::env()),
            ("env.is_prod", &Env::is_prod()),
            ("env.is_dev", &Env::is_dev()),
            ("env.is_test", &Env::is_test()),
            ("env.is_debug", &Env::is_debug()),
            ("env.is_release", &Env::is_release())
        ]
        .iter()
        .map(convert::tuple_string)
        .collect()
    }
}

impl TryFrom<&Di> for AppConfig {
    type Error = Err;

    fn try_from(di: &Di) -> Result<Self, Self::Error> {
        let mut config = Self::default();
        let dirs = di.get::<Dirs>()?;
        let args = di.get::<CmdArgs>()?;

        config.extend(
            [
                ("config_file", args.get_opt(Self::OPT_CONFIG_FILE)),
                ("env_file", args.get_opt(Self::OPT_ENV_FILE)),
                ("pid_file", args.get_opt(Self::OPT_PID_FILE))
            ]
            .iter()
            .map(convert::tuple_option_str)
        );

        if config.config_file.is_empty() == false
            && config.config_file.starts_with(['/', '.']) == false
        {
            config.config_file =
                format!("{}/{}", &dirs.config, &config.config_file).into();
        }

        if config.pid_file.is_empty() == false
            && config.pid_file.starts_with(['/', '.']) == false
        {
            config.pid_file = format!("{}/{}", &dirs.run, &config.pid_file).into();
        }

        if let Some(env_file) = config.env_file.as_mut()
            && env_file.is_empty() == false
            && env_file.starts_with(['/']) == false
        {
            *env_file = format!("{}/{}", &dirs.data, env_file).into()
        }

        Ok(config)
    }
}
