use {
    app_base::prelude::*,
    app_kernel::prelude::*,
    core::ptr::fn_addr_eq,
    criterion::{Criterion, criterion_group, criterion_main}
};

static MOD_EXAMPLE: AppModule = SimpleModule::module;

const NT_TEST: &str = "test";

#[derive(Default)]
struct SimpleModule;

impl AppModuleExt for SimpleModule {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_EXAMPLE),
        module: Self::module,
        depends: [].into(),
        sends: [].into(),
        notifies: [NT_APP_RUN, NT_TEST].into(),
        hooks: [].into(),
        commands: [].into()
    });

    fn notify(&self, app: &App, event: &AppEventData) -> Void {
        match event.notify {
            NT_APP_RUN => {
                let result = app
                    .notify(NT_TEST, Some(self), Some(&"Hello, World!"), None)?
                    .map(|r| r.downcast::<&str>().ok())
                    .flatten();

                assert_eq!("This is result!", *result.unwrap());
            },
            NT_TEST => {
                event.set_result("This is result!")?;
            },
            _ => ()
        }
        .void()
    }

    fn hook(
        &self,
        _app: &App,
        event: &AppEventData,
        _hook_event: &AppEventData,
        is_pre_hook: bool
    ) -> Void {
        match (event.event, event.notify) {
            (AppEvent::NOTIFY, NT_APP_RUN) if is_pre_hook => {
                assert_eq!("App context", *event.context_as::<&str>()?);
            },
            (AppEvent::NOTIFY, NT_TEST) if is_pre_hook == false => {
                assert_eq!("This is result!", *event.result_as::<&str>()?.unwrap());
                assert_eq!("Hello, World!", *event.context_as::<&str>()?);
            },
            (AppEvent::LOAD, "") | (AppEvent::UNLOAD, "") => {
                assert!(fn_addr_eq(MOD_EXAMPLE, *event.sender_as::<AppModule>()?));
            },
            _ => ()
        }
        .void()
    }
}

fn app_bench(c: &mut Criterion) {
    let app = App::new(&[MOD_EXAMPLE]).unwrap();

    c.bench_function("App::notify(NT_TEST) - no hooks", |b| {
        b.iter(|| {
            app.notify(NT_TEST, None, Some(&"Hello, World!"), None)
                .unwrap();
        })
    });

    c.bench_function("App::run() - no hooks", |b| {
        b.iter(|| {
            app.run(Some(&"App context")).unwrap();
        })
    });

    app.add_hook_handler(AppEvent::NOTIFY, NT_APP_RUN, MOD_EXAMPLE)
        .unwrap();
    app.add_hook_handler(AppEvent::NOTIFY, NT_TEST, MOD_EXAMPLE)
        .unwrap();

    c.bench_function("App::notify(NT_TEST) - with hooks", |b| {
        b.iter(|| {
            app.notify(NT_TEST, None, Some(&"Hello, World!"), None)
                .unwrap();
        })
    });

    c.bench_function("App::run() - with hooks", |b| {
        b.iter(|| {
            app.run(Some(&"App context")).unwrap();
        })
    });
}

criterion_group!(benches, app_bench);
criterion_main!(benches);
