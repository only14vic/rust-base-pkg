#![cfg_attr(not(feature = "std"), no_std)]
#![no_main]

#[allow(unused_imports)]
#[macro_use]
extern crate alloc;
extern crate core;

#[cfg(not(feature = "std"))]
use core::ffi::{c_char, c_int};

use {app_base::prelude::*, app_kernel::prelude::*};

#[unsafe(no_mangle)]
fn main(
    #[cfg(not(feature = "std"))] argc: c_int,
    #[cfg(not(feature = "std"))] argv: *const *const c_char
) -> Void {
    dotenv(true);
    log_init();

    let result = App::new(
        &[MOD_BAR],
        #[cfg(not(feature = "std"))]
        argc,
        #[cfg(not(feature = "std"))]
        argv
    )?
    .run(Some(&"Run context data"))?;

    dbg!(result.map(|r| r.downcast::<&str>()));

    mem_stats();

    ok()
}

const MOD_FOO: AppModule = Foo::module;
const MOD_BAR: AppModule = Bar::module;
const MOD_ZAR: AppModule = Zar::module;

#[derive(Default, Debug)]
struct Foo;

impl AppModuleExt for Foo {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_FOO),
        module: Self::module,
        depends: [MOD_ZAR].into(),
        sends: [].into(),
        notifies: [NT_APP_RUN, "Foo::run"].into(),
        hooks: [
            &AppEvent::LOAD as &dyn AppHook,
            &AppEvent::UNLOAD,
            &NT_APP_RUN
        ]
        .into(),
        commands: [].into()
    });

    fn hook(
        &self,
        _app: &App,
        event: &AppEventData,
        _hook_event: &AppEventData,
        is_pre_hook: bool
    ) -> Void {
        println!(
            "For catch hook: {is_pre_hook:?} {} {:?}",
            event.event, event.notify
        );
        ok()
    }

    fn notify(&self, _app: &App, event: &AppEventData) -> Void {
        println!(
            "Foo catch notify: {} {:?} {:?}",
            event.notify,
            event.sender_as::<Zar>(),
            event.context_as::<&str>()
        );

        match event.notify {
            NT_APP_RUN => {
                event.set_result("Set Result of App")?;
            },
            "Foo::run" => {
                event.set_result("Foo set result")?;
                dbg!("Foo running");
            },
            _ => ()
        }
        ok()
    }
}

impl Drop for Foo {
    fn drop(&mut self) {
        println!("Drop: Foo");
    }
}

#[allow(dead_code)]
#[derive(Default, Debug)]
struct Bar;

impl AppModuleExt for Bar {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_BAR),
        module: Self::module,
        depends: [MOD_FOO].into(),
        sends: [].into(),
        notifies: ["Bar::run"].into(),
        hooks: [
            &AppEvent::LOAD as &dyn AppHook,
            &AppEvent::UNLOAD,
            &"Zar::run"
        ]
        .into(),
        commands: [].into()
    });

    fn hook(
        &self,
        _app: &App,
        event: &AppEventData,
        _hook_event: &AppEventData,
        is_pre_hook: bool
    ) -> Void {
        println!(
            "Bar catch hook: {is_pre_hook:?} {} {:?}",
            event.event, event.notify
        );
        ok()
    }

    fn notify(&self, app: &App, event: &AppEventData) -> Void {
        println!(
            "Bar catch notify: {} {:?} {:?}",
            event.notify,
            event.sender_as::<Zar>(),
            event.context_as::<&str>()
        );

        match event.notify {
            "Bar::run" => {
                event.set_result("Bar set result")?;
                _ = dbg!(
                    app.notify("Zar::run", Some(self), Some(&"Zar event data"), None)?
                        .map(|c| c.downcast::<&str>())
                );
            },
            _ => ()
        }

        ok()
    }
}

impl Drop for Bar {
    fn drop(&mut self) {
        println!("Drop: Bar");
    }
}

#[derive(Default, Debug)]
struct Zar;

impl AppModuleExt for Zar {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_ZAR),
        module: Self::module,
        depends: [MOD_FOO].into(),
        sends: [].into(),
        notifies: [NT_APP_RUN, "Zar::run"].into(),
        hooks: [&AppEvent::LOAD as &dyn AppHook, &AppEvent::UNLOAD].into(),
        commands: [].into()
    });

    fn hook(
        &self,
        _app: &App,
        event: &AppEventData,
        _hook_event: &AppEventData,
        is_pre_hook: bool
    ) -> Void {
        println!(
            "Zar catch hook: {is_pre_hook:?} {} {:?}",
            event.event, event.notify
        );
        ok()
    }

    fn notify(&self, app: &App, event: &AppEventData) -> Void {
        println!(
            "Zar catch notify: {} {:?}",
            event.notify,
            event.context_as::<&str>()
        );

        match event.notify {
            NT_APP_RUN => {
                _ = dbg!(
                    app.notify("Bar::run", Some(self), Some(&"Bar event data"), None)?
                        .map(|c| c.downcast::<&str>())
                );
            },
            "Zar::run" => {
                event.set_result("Zar set result")?;
                _ = dbg!(
                    app.notify("Foo::run", Some(self), Some(&"Foo event data"), None)?
                        .map(|c| c.downcast::<&str>())
                );
            },
            _ => ()
        }
        ok()
    }
}

impl Drop for Zar {
    fn drop(&mut self) {
        println!("Drop: Zar");
    }
}
