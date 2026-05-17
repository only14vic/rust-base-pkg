use {alloc::format, app_base::prelude::*, core::ptr::fn_addr_eq};

mod cmd_args_ext;
mod cmd_config;

pub use {crate::*, cmd_args_ext::*, cmd_config::*};

pub static MOD_CMD: AppModule = CmdMod::module;

#[derive(Default)]
struct CmdMod;

impl CmdMod {
    fn set_default_cmd(&self, app: &App) -> Void {
        let cmd_config = app.get::<CmdConfig>()?;

        if let Some(def_cmd) = cmd_config.default.as_ref() {
            CmdArgs::set_default_cmd(def_cmd);
        }

        ok()
    }

    fn run_bin_cmd(&self, app: &App) -> Void {
        let args = app.get::<CmdArgs>()?;
        let cmd = args.get_cmd().unwrap_or_default();

        if cmd.is_empty() {
            return ok();
        }

        let config = app.get::<CmdConfig>()?;
        let mut cmd_bin = None;
        for (bin, aliases) in config.bins.iter() {
            if aliases.contains(&cmd) && bin.is_empty() == false {
                cmd_bin = Some(bin);
            }
        }

        let dirs = app.get::<Dirs>()?;
        let bin = format!(
            "{}/{}-{}",
            &dirs.bin,
            &dirs.exe_file(),
            cmd_bin.unwrap_or(&cmd)
        );

        let cmd_args_line = app.get::<CmdArgsLine>()?.read().args();
        let bin_args = cmd_args_line.get(1..).unwrap_or_default();
        let bin_args = match bin_args.splitn(2, |v| v == "--").nth(1) {
            Some(args) => args.to_vec(),
            None => {
                bin_args
                    .iter()
                    .filter(|v| v.as_str() != cmd.as_str())
                    .cloned()
                    .collect()
            },
        };

        if Dirs::exists(&bin) {
            Env::is_debug()
                .then(|| log::debug!("Running bin: {bin} {}", bin_args.join(" ")));

            #[cfg(feature = "std")]
            return {
                std::process::Command::new(&bin)
                    .args(bin_args)
                    .spawn()
                    .expect(&format!("Failed execution: {bin}"))
                    .wait()
                    .and_then(|r| {
                        if r.success() {
                            ok()
                        } else {
                            Err(std::io::Error::other(format!("Failed execution: {bin}")))
                        }
                    })?
                    .into_ok()
            };

            #[cfg(not(feature = "std"))]
            return unsafe {
                use alloc::ffi::CString;

                let bin = format!("{} {}", bin, bin_args.join(" "));
                let bin_c = CString::new(bin.as_str())?;
                match libc::system(bin_c.as_ptr()) {
                    0 => ok(),
                    _ => Err(format!("Failed execution: {bin}"))?
                }
            };
        }

        return Err(format!("Invalid command: {cmd}"))?;
    }
}

impl AppModuleExt for CmdMod {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_CMD, no_mangle, #[cfg(feature = "bind")]),
        module: Self::module,
        #[rustfmt::skip]
        depends: [
            MOD_LOG,
            MOD_HELP,
            MOD_CONFIG,
            #[cfg(not(target_env = "musl"))]
            MOD_LD
        ].into(),
        notifies: [NT_CONFIG_DISPLAY].into(),
        sends: [].into(),
        hooks: [&NT_APP_RUN as &dyn AppHook, &AppEvent::LOAD].into(),
        commands: [].into()
    });

    fn init(&mut self, app: &App, _event: &AppEventData) -> Void {
        app.get::<CmdArgsLine>()?
            .write()
            .add_flags(&[
                AppConfig::OPT_VERSION,
                AppConfig::OPT_HELP,
                AppConfig::OPT_VERBOSE,
                AppConfig::OPT_DEBUG,
                AppConfig::OPT_FORCE
            ])?
            .add_alias(&[
                AppConfig::OPT_ENV_FILE,
                AppConfig::OPT_CONFIG_FILE,
                AppConfig::OPT_PID_FILE
            ])?;

        app.add_creator::<CmdArgs>(|di| {
            let cmd_args_line = di.get::<CmdArgsLine>()?;
            Ok(CmdArgs::parse(&cmd_args_line.read()))
        });

        ok()
    }

    fn notify(&self, app: &App, event: &AppEventData) -> Void {
        if event.notify == NT_CONFIG_DISPLAY {
            app.get::<ConfigDisplay>()?
                .write()
                .push(app.get::<CmdConfig>()?);
        }

        ok()
    }

    fn hook(
        &self,
        app: &App,
        event: &AppEventData,
        _hook_event: &AppEventData,
        is_pre_hook: bool
    ) -> Void {
        if event.event == AppEvent::LOAD {
            // Remove CmdArgs to parse again
            app.remove::<CmdArgs>();
        }

        if event.event == AppEvent::LOAD
            && is_pre_hook == false
            && fn_addr_eq(*event.sender_as::<AppModule>()?, MOD_CONFIG)
            && CmdArgs::get_default_cmd().is_none()
        {
            self.set_default_cmd(app)?;
        }

        if event.notify == NT_APP_RUN && is_pre_hook == false {
            if event.handled.get() {
                return ok();
            }

            let cmd = app.get::<CmdArgs>()?.get_cmd().unwrap_or_default();
            if cmd.is_empty() {
                return ok();
            }

            event.handled.set(true);
            self.run_bin_cmd(app)?;
        }

        ok()
    }
}
