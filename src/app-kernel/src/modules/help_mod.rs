use {
    crate::{modules::CmdArgsExt, prelude::*},
    alloc::{
        format,
        string::{String, ToString},
        vec::Vec
    },
    app_base::prelude::*,
    compression::prelude::*,
    core::fmt::Display
};

pub static MOD_HELP: AppModule = HelpMod::module;

pub const NT_CMD_HELP: &str = "Cmd::help";
pub const NT_CMD_VERSION: &str = "Cmd::version";

#[derive(Default)]
pub struct HelpContext {
    /// Map of: `command`=`description`
    pub commands: RwLock<IndexMap<String, String>>,
    /// Map of: `name`=`value`
    pub params: RwLock<IndexMap<String, String>>
}

#[derive(Default)]
pub struct HelpMod;

impl HelpMod {
    const HELP_TEXT: &[u8] = include_bytes!(env!("HELP_FILE"));

    fn show_version(&self, app: &App) -> Void {
        let config = app.get::<AppConfig>()?;
        println!(
            "{} {} {}",
            config.name,
            config.version,
            if config.no_std { "[no_std]" } else { Default::default() }
        );
        ok()
    }

    fn show_help(&self, app: &App, context: &HelpContext) -> Void {
        let args = app.get::<CmdArgs>()?;

        let cmd = args.get_cmd().unwrap_or_default();
        let sub_cmd = args.get_sub_cmd().unwrap_or_default();

        let help_id = "#HELP:";
        let cmd_and_sub = if cmd.is_empty() {
            "".into()
        } else if sub_cmd.is_empty() {
            cmd.to_string()
        } else {
            format!("{cmd} {sub_cmd}")
        };
        let mut found_str = "";
        let mut found = false;

        let text = String::from_utf8(
            Self::HELP_TEXT
                .to_vec()
                .decode(&mut ZlibDecoder::new())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        )?;
        let iter = text.split(&help_id);

        'main: for s in iter {
            if s.trim().is_empty() {
                continue;
            }

            let end_info = s.find('\n').unwrap_or(s.len());
            let item_str = s[end_info..].trim();

            let names = s[0..end_info]
                .trim()
                .split(',')
                .filter(|s| s.is_empty() == false)
                .map(|s| s.trim())
                .collect::<Vec<_>>();

            if names.is_empty() && found == false {
                found_str = item_str;
                if cmd.is_empty() {
                    found = true;
                    break 'main;
                }
                continue;
            }

            let names_iter = names.into_iter();
            for name in names_iter {
                if name == cmd.as_str() && found == false {
                    found_str = item_str;
                    found = true;
                }
                if name == cmd_and_sub.as_str() {
                    found_str = item_str;
                    found = true;
                    break 'main;
                }
            }
        }

        let commands = &context.commands;
        let commands = commands.read();

        let len = commands.iter().map(|(c, _)| c.len()).max().unwrap_or(0);
        let mut commands_list = String::default();
        for (cmd, desc) in commands.iter() {
            commands_list.push_str(&format!("{:<4}{cmd:<len$} - {desc}\n", " "));
        }

        let config = app.get::<AppConfig>()?;
        let params: [(&str, &dyn Display); _] = [
            ("{bin}", &args.get_exe_file()),
            ("{name}", &config.name),
            ("{version}", &config.version),
            ("{cmd}", &cmd),
            ("{sub_cmd}", &sub_cmd),
            ("{commands}", &commands_list.trim_end())
        ];

        let mut help = found_str.to_string();

        for (n, v) in params {
            help = help.replace(n, &v.to_string());
        }
        for (n, v) in context.params.read().iter() {
            help = help.replace(n, v);
        }

        println!("{help}");

        if found == false {
            return Err(format!("Help not found for command: {cmd}"))?;
        }

        ok()
    }
}

impl AppModuleExt for HelpMod {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_HELP, no_mangle, #[cfg(feature = "bind")]),
        module: Self::module,
        depends: [MOD_CONFIG, MOD_CMD].into(),
        notifies: [NT_CMD_VERSION].into(),
        sends: [NT_CMD_HELP, NT_CMD_VERSION].into(),
        hooks: [&NT_APP_RUN as &dyn AppHook].into(),
        commands: [].into()
    });

    fn notify(&self, app: &App, event: &AppEventData) -> Void {
        if event.notify == NT_CMD_VERSION {
            event.handled.set(true);
            self.show_version(app)?;
        }

        if event.notify == NT_CMD_HELP {
            event.handled.set(true);
            let context = event.context_as::<HelpContext>()?;
            self.show_help(app, context)?;
        }

        ok()
    }

    fn hook(
        &self,
        app: &App,
        event: &AppEventData,
        hook_event: &AppEventData,
        is_pre_hook: bool
    ) -> Void {
        if event.notify == NT_APP_RUN && is_pre_hook == true {
            let args = app.get::<CmdArgs>()?;

            // Last handler
            app.add_notify_handler(NT_CMD_HELP, Self::module)?;

            if args.do_version() {
                event.handled.set(true);
                hook_event.handled.set(true);

                app.notify(NT_CMD_VERSION, Some(self), None, None)?;
            } else if args.do_help() {
                event.handled.set(true);
                hook_event.handled.set(true);

                app.notify(NT_CMD_HELP, Some(self), Some(&HelpContext::default()), None)?;
            }
        }

        if event.notify == NT_APP_RUN && is_pre_hook == false {
            if event.handled.get() {
                return ok();
            }

            app.notify(NT_CMD_HELP, Some(self), Some(&HelpContext::default()), None)?;
        }

        ok()
    }
}
