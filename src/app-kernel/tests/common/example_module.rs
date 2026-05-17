extern crate alloc;
extern crate core;

use {
    alloc::boxed::Box,
    app_base::prelude::*,
    app_kernel::{modules::*, prelude::*}
};

pub static MOD_EXAMPLE: AppModule = ExampleModule::module;

pub const NT_TEST: &str = "test";

#[derive(Default)]
struct ExampleModule;

impl AppModuleExt for ExampleModule {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_EXAMPLE, no_mangle),
        module: Self::module,
        depends: [
            MOD_CMD,
            Self::module /* checks there is no recursive */
        ]
        .into(),
        sends: [].into(),
        notifies: [NT_APP_RUN, NT_TEST].into(),
        hooks: [
            &NT_APP_RUN as &dyn AppHook,
            &NT_TEST,
            Box::leak(Box::new((AppEvent::LOAD, MOD_CMD))),
            Box::leak(Box::new((AppEvent::LOAD, MOD_CONFIG))),
            Box::leak(Box::new((AppEvent::LOAD, MOD_EXAMPLE))),
            Box::leak(Box::new((AppEvent::UNLOAD, MOD_EXAMPLE)))
        ]
        .into(),
        commands: [].into()
    });

    fn init(&mut self, _app: &App, _event: &AppEventData) -> Void {
        println!("INIT");
        println!("NOTIFIES: {:?}", Self::meta().notifies);
        println!("HOOKS: {:?}", Self::meta().hooks);
        ok()
    }

    fn boot(&mut self, app: &App, _event: &AppEventData) -> Void {
        println!("BOOT");
        println!("{}", app.get::<ConfigDisplay>()?);
        ok()
    }

    fn notify(&self, app: &App, event: &AppEventData) -> Void {
        Env::is_debug().then(|| println!("NOTIFY {:?}", event.notify));

        match event.notify {
            NT_APP_RUN => {
                event.handled.set(true);

                _ = dbg!(event.context_as::<&str>());
                event.set_result("App result!")?;

                println!("Self address: {self:p}");
                assert!(
                    dbg!(app.notify("foo", Some(self), Some(&Self::module), None))
                        .is_ok()
                );

                let context = "Hello, World!";
                println!("Context address: {context:p}");

                let max = if Env::is_debug() { 1 } else { 1_000_000 };
                println!("Sending {max} notifies");

                for _ in 0..max {
                    assert_eq!(
                        "This is result!",
                        *app.notify(NT_TEST, Some(self), Some(&context), None)?
                            .map(|r| r.downcast::<&str>().unwrap())
                            .unwrap()
                    );
                }
            },
            NT_TEST => {
                assert_eq!("Hello, World!", *event.context_as::<&str>()?);
                event.set_result("This is result!")?;
            },
            _ => ()
        }
        ok()
    }

    fn hook(
        &self,
        _app: &App,
        event: &AppEventData,
        _hook_event: &AppEventData,
        is_pre_hook: bool
    ) -> Void {
        Env::is_debug()
            .then(|| println!("HOOK {is_pre_hook} {}: {:?}", event.event, event.notify));

        match (event.event, event.notify) {
            (AppEvent::NOTIFY, NT_APP_RUN) if is_pre_hook => {
                println!("Hook {is_pre_hook} {}", event.notify);
            },
            (AppEvent::NOTIFY, NT_TEST) if is_pre_hook == false => {
                assert_eq!("This is result!", *event.result_as::<&str>()?.unwrap());
                assert_eq!("Hello, World!", *event.context_as::<&str>()?);
            },
            (AppEvent::LOAD, module_addr) | (AppEvent::UNLOAD, module_addr) => {
                println!(
                    "Hook {} {is_pre_hook:?} {module_addr:?} {:p}",
                    event.event,
                    *event.sender_as::<AppModule>()?
                );
            },
            _ => ()
        }

        ok()
    }

    fn down(&mut self, _app: &App, _event: &AppEventData) -> Void {
        println!("UNLOAD");
        ok()
    }
}
