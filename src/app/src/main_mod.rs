use {crate::*, app_common::db::MOD_DB, app_kernel::prelude::*};

pub static MOD_MAIN: AppModule = MainMod::module;

#[derive(Default)]
struct MainMod;

impl AppModuleExt for MainMod {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_MAIN, no_mangle),
        module: Self::module,
        depends: [MOD_DB, MOD_ASYNC, MOD_CMD].into(),
        sends: [].into(),
        notifies: [NT_CONFIG_DISPLAY, NT_APP_RUN].into(),
        hooks: [].into(),
        commands: ["main"].into()
    });

    fn boot(&mut self, app: &App, _event: &AppEventData) -> Void {
        let args = app.get::<CmdArgs>()?;

        if args.do_help() == false && args.do_version() == false {
            let dirs = app.get::<Dirs>()?;
            dirs.create_dirs()?;
        }

        ok()
    }

    fn notify(&self, app: &App, event: &AppEventData) -> Void {
        let cmd = app.get::<CmdArgs>()?.get_cmd().unwrap_or_default();

        match event.notify {
            NT_APP_RUN if Self::meta().commands.contains(&cmd.as_str()) => {
                event.handled.set(true);
                app.notify(NT_ASYNC_START, Some(self), Some(&true), None)?;
            },
            NT_CONFIG_DISPLAY => {
                app.get::<ConfigDisplay>()?
                    .write()
                    .push(app.get::<MainConfig>()?);
            },
            _ => ()
        };

        ok()
    }
}
