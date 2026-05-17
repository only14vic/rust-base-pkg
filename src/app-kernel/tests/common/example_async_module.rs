use {
    app_base::prelude::*,
    app_kernel::prelude::*,
    core::time::Duration,
    futures::executor::block_on,
    tokio::{spawn, time::sleep}
};

#[allow(unused)]
pub const MOD_ASYNC_EXAMPLE: AppModule = ExampleAsyncModule::module;

#[allow(unused)]
#[derive(Default)]
pub struct ExampleAsyncModule;

impl AppModuleExt for ExampleAsyncModule {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_ASYNC_EXAMPLE, no_mangle),
        module: Self::module,
        depends: [MOD_ASYNC].into(),
        sends: [].into(),
        notifies: [NT_APP_RUN].into(),
        hooks: [].into(),
        commands: [].into()
    });

    fn init(&mut self, app: &App, _event: &AppEventData) -> Void {
        app.get::<AsyncInit>()?.add(async |_di| {
            println!("Async initialized!");
            ok()
        });
        ok()
    }

    fn notify(&self, _app: &App, event: &AppEventData) -> Void {
        event.handled.set(true);

        const MAX: u64 = 10;

        for i in 0..MAX {
            spawn(async move {
                sleep(Duration::from_millis(MAX - i)).await;
                println!("Hello from Async: {i}");
            });
        }

        block_on(async { sleep(Duration::from_millis(MAX)).await });

        ok()
    }
}
