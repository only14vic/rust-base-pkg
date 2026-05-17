use {
    crate::{modules::*, prelude::*},
    app_base::prelude::*
};

pub static MOD_ENV: AppModule = EnvMod::module;

#[derive(Default)]
struct EnvMod;

impl AppModuleExt for EnvMod {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_ENV, no_mangle, #[cfg(feature = "bind")]),
        module: Self::module,
        depends: [MOD_CONFIG, MOD_CMD].into(),
        notifies: [].into(),
        sends: [].into(),
        hooks: [].into(),
        commands: [].into()
    });

    fn init(&mut self, _app: &App, _event: &AppEventData) -> Void {
        dotenv(false);
        ok()
    }

    fn boot(&mut self, app: &App, _event: &AppEventData) -> Void {
        let config = app.get::<AppConfig>()?;
        if let Some(env_file) = config.env_file.as_ref() {
            Env::load(env_file, true)?;
        }

        let args = app.get::<CmdArgs>()?;
        if args.is_debug() {
            setenv("APP_DEBUG", "1");
            unsafe { Env::reset() };
        }

        ok()
    }
}
