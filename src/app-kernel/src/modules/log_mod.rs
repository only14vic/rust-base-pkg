use {
    crate::{
        modules::{MOD_CONFIG, MOD_ENV},
        prelude::*
    },
    app_base::prelude::*
};

pub static MOD_LOG: AppModule = LogMod::module;

#[derive(Default)]
struct LogMod;

impl AppModuleExt for LogMod {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_LOG, no_mangle, #[cfg(feature = "bind")]),
        module: Self::module,
        depends: [MOD_ENV, MOD_CONFIG].into(),
        sends: [].into(),
        notifies: [].into(),
        hooks: [].into(),
        commands: [].into()
    });

    fn init(&mut self, app: &App, _event: &AppEventData) -> Void {
        let logger = log_init();
        app.add(logger);

        ok()
    }

    fn boot(&mut self, app: &App, _event: &AppEventData) -> Void {
        let config = app.get::<LogConfig>()?;
        let logger = unsafe { Logger::from_static_mut() };
        logger.configure(&config)?;

        ok()
    }

    fn down(&mut self, _app: &App, _event: &AppEventData) -> Void {
        unsafe { Logger::from_static_mut() }.log_close();

        ok()
    }
}
