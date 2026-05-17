use {
    app_base::prelude::*,
    app_common::db::DbExec,
    app_kernel::prelude::*,
    core::{any::TypeId, time::Duration},
    sqlx::{Postgres, Row, postgres::PgRow},
    std::io::{Write, stdout},
    tokio::{select, spawn, time::sleep},
    yansi::Paint
};

extern crate core;
extern crate alloc;

pub static MOD_A: AppModule = ModuleA::module;
#[unsafe(no_mangle)]
pub static NT_MOD_A_PING: &str = "ModuleA::ping";

#[link(name = "app_b")]
#[allow(improper_ctypes)]
unsafe extern "C" {
    static NT_MOD_B_PING: &'static str;
    static MOD_B: AppModule;
}

#[derive(Default)]
pub struct ModuleA;

impl AppModuleExt for ModuleA {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_A, no_mangle),
        module: Self::module,
        depends: [unsafe { MOD_B }].into(),
        sends: [].into(),
        notifies: [NT_MOD_A_PING, NT_ASYNC_STARTED, NT_APP_RUN].into(),
        hooks: [].into(),
        commands: ["a"].into()
    });

    fn init(&mut self, _app: &App, _event: &AppEventData) -> Void {
        log::info!("ModuleA init. App type id: {:#?}", TypeId::of::<App>());
        ok()
    }

    fn notify(&self, app: &App, event: &AppEventData) -> Void {
        let cmd = app.get::<CmdArgs>()?.get_cmd().unwrap_or_default();

        if event.notify == NT_MOD_A_PING {
            spawn(async {
                print!("{}", char::from_u32(0x2665).unwrap().red());
                stdout().flush()?;

                ok() as VoidSync
            });

            let db_exec = app.get::<DbExec>()?;

            spawn(App::error_handler_async(
                async move {
                    let query = sqlx::query::<Postgres>("select $1")
                        .bind(char::from_u32(0x26a1).unwrap().to_string());
                    let res = db_exec.exec::<Vec<PgRow>>(query).await?;
                    let item: String = res[0].get(0);

                    print!("{}", item.blue());
                    stdout().flush()?;

                    ok() as VoidSync
                },
                || module_path!()
            ));
        }

        if event.notify == NT_ASYNC_STARTED {
            tokio_start_custom(app, "tokio-mod-a", 1)?;

            let stop = (**app.get::<AsyncStopSignal>()?).clone();
            let app = app.get_weak::<App>()?.upgrade().unwrap();
            #[allow(unreachable_code)]
            let fut = async move {
                let mut i = 0u64;
                loop {
                    app.notify(NT_MOD_A_PING, None, None, None)?;

                    let app_ref = app.clone();
                    spawn(async move {
                        app_ref.notify(unsafe { NT_MOD_B_PING }, None, None, None)?;
                        ok() as VoidSync
                    });

                    i += 1;

                    if i < 10_000 {
                        sleep(Duration::from_millis(1)).await;
                    } else {
                        if i > 10_500 {
                            i = 0;
                        }
                        sleep(Duration::from_millis(100)).await;
                    }
                }

                ok() as VoidSync
            };

            spawn(App::error_handler_async(
                async move {
                    select! {
                        res = fut => res,
                        _ = stop => ok()
                    }
                },
                || module_path!()
            ));
        }

        if event.notify == NT_APP_RUN && Self::meta().commands.contains(&cmd.as_str()) {
            event.handled.set(true);
            app.notify(NT_ASYNC_START, None, Some(&true), None)?;
        }
        ok()
    }
}

ld_meta!(LdMeta {
    lib: module_path!().into(),
    mods: [App::meta(MOD_A).clone()].into()
});
