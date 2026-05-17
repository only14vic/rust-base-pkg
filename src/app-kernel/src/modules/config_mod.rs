use {
    crate::{modules::*, prelude::*},
    alloc::{format, string::ToString, sync::Arc, vec::Vec},
    app_base::prelude::*
};

mod config_display;

pub use config_display::*;

pub static MOD_CONFIG: AppModule = ConfigMod::module;
pub const NT_CONFIG_DISPLAY: &str = "Config::config_display";

#[derive(Default)]
struct ConfigMod;

impl ConfigMod {
    const OPT_CONFIG: &str = "c:config";
    const OPT_ARGUMENTS: &str = "a:arguments";
    const OPT_APPLICATION: &str = "A:application";
    const OPT_MODULE_META: &str = "m:module-meta";

    fn run(&self, app: &App) -> Void {
        let args = app.get::<CmdArgs>()?;
        let args_line = app.get::<CmdArgsLine>()?;

        if args.get_flag(Self::OPT_ARGUMENTS) {
            println!("Flags: {:#?}", args_line.read().flags);
            println!("Aliases: {:#?}", args.alias);
            println!("Arguments: {:#?}", args.args);
            println!("Options: {:#?}", args.argv);
            println!("Executable: {:?}", args.get_exe());
            println!("Command: {:?}", args.get_cmd());
            println!("Subcommand: {:?}", args.get_sub_cmd());
            return ok();
        }

        if args.get_flag(Self::OPT_APPLICATION) {
            println!("{app:#?}");
            return ok();
        }

        if let Some(module_names) = args.get_opt_list(Self::OPT_MODULE_META) {
            for module in app.modules() {
                let meta = App::meta(module);
                if module_names.is_empty()
                    || module_names.iter().any(|n| {
                        meta.name == n.to_ascii_uppercase()
                            || meta.name_short() == n.to_ascii_lowercase()
                    })
                {
                    println!("{meta:#?}");

                    if module_names.is_empty() == false {
                        return ok();
                    }
                }
            }

            if module_names.is_empty() {
                return ok();
            } else {
                return Err(format!(
                    "Module not loaded: {}",
                    module_names.first().unwrap()
                ))?;
            }
        }

        let mut list = if args.get_flag(Self::OPT_CONFIG) {
            app.get::<ConfigOptions>()?.to_string()
        } else {
            app.get::<ConfigDisplay>()?.to_string()
        };

        if let Some(filter) = args.get_arg(2) {
            list = list
                .trim()
                .lines()
                .filter(|s| s.contains(filter))
                .collect::<Vec<_>>()
                .join("\n");

            if list.lines().count() == 1
                && list.starts_with(filter)
                && list.chars().nth(filter.len()) == Some('=')
            {
                list.replace_range(..=filter.len(), "");
            }
        }

        println!("{list}");

        ok()
    }
}

impl AppModuleExt for ConfigMod {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_CONFIG, no_mangle, #[cfg(feature = "bind")]),
        module: Self::module,
        depends: [MOD_CMD].into(),
        notifies: [NT_APP_RUN, NT_APP_SHUTDOWN, NT_CONFIG_DISPLAY].into(),
        sends: [NT_CONFIG_DISPLAY].into(),
        hooks: [&NT_APP_RUN as &dyn AppHook].into(),
        commands: ["c", "config"].into()
    });

    fn init(&mut self, app: &App, _event: &AppEventData) -> Void {
        let cmd = app.get::<CmdArgs>()?.get_cmd().unwrap_or_default();
        let cmd_args_line = app.get::<CmdArgsLine>()?;

        cmd_args_line.write().add_flags(&[Self::OPT_APPLICATION])?;

        if Self::meta().commands.contains(&cmd.as_str()) {
            cmd_args_line
                .write()
                .add_flags(&[Self::OPT_CONFIG, Self::OPT_ARGUMENTS])?
                .add_alias(&[Self::OPT_MODULE_META])?;
        }

        ok()
    }

    fn boot(&mut self, app: &App, _event: &AppEventData) -> Void {
        let dirs = app.get::<Dirs>()?;
        let config = app.get::<AppConfig>()?;
        let config_file = &config.config_file;

        let mut ini = match Ini::from_file(&config_file) {
            Ok(ini) => {
                Env::is_debug().then(|| log::debug!("Loading: {config_file}"));
                ini
            },
            Err(e) => {
                match e.downcast_ref::<IniError>() {
                    Some(IniError::FileNotFound(..)) => Ini::default(),
                    _ => Err(e)?
                }
            },
        };

        let user_config_file =
            format!("{}/{}", &dirs.user_config, Dirs::basename(config_file));

        match Ini::from_file(&user_config_file) {
            Ok(user_ini) => {
                Env::is_debug().then(|| log::debug!("Loading: {user_config_file}"));
                ini.extend(
                    user_ini
                        .into_iter()
                        .map(|(n, v)| (n.into(), v.map(|v| v.into())))
                );
            },
            Err(e) => {
                match e.downcast_ref::<IniError>() {
                    Some(IniError::FileNotFound(..)) => (),
                    _ => Err(e)?
                }
            },
        };

        app.add(ConfigOptions::new(ini));

        ok()
    }

    fn notify(&self, app: &App, event: &AppEventData) -> Void {
        let args = app.get::<CmdArgs>()?;
        let cmd = args.get_cmd().unwrap_or_default();

        if event.notify == NT_APP_RUN && Self::meta().commands.contains(&cmd.as_str()) {
            event.handled.set(true);
            let config_display = app.get::<ConfigDisplay>()?;
            app.notify(NT_CONFIG_DISPLAY, Some(self), Some(&*config_display), None)?;
            self.run(app)?;
        }

        if event.notify == NT_APP_SHUTDOWN
            && args.get_flag(Self::OPT_APPLICATION)
            && Self::meta().commands.contains(&cmd.as_str()) == false
        {
            println!("{app:#?}");
        }

        if event.notify == NT_CONFIG_DISPLAY {
            app.get::<ConfigDisplay>()?.write().extend([
                app.get::<BaseConfig>()? as Arc<dyn IterConfig>,
                app.get::<LogConfig>()?,
                app.get::<AppConfig>()?,
                app.get::<Dirs>()?
            ]);
        }

        ok()
    }
}
