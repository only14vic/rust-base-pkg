#![cfg_attr(not(feature = "std"), no_std)]

use {
    alloc::format,
    app_base::prelude::*,
    app_kernel::prelude::*,
    core::{any::TypeId, ptr::null},
    yansi::Paint
};

extern crate core;
extern crate alloc;

pub static MOD_B: AppModule = ModuleB::module;
#[unsafe(no_mangle)]
pub static NT_MOD_B_PING: &str = "ModuleB::ping";

#[derive(Default)]
pub struct ModuleB;

impl AppModuleExt for ModuleB {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_B, no_mangle),
        module: Self::module,
        depends: [].into(),
        sends: [].into(),
        notifies: [NT_MOD_B_PING, NT_APP_RUN].into(),
        hooks: [].into(),
        commands: ["b"].into()
    });

    fn init(&mut self, _app: &App, _event: &AppEventData) -> Void {
        log::info!("ModuleB init. App type id: {:#?}", TypeId::of::<App>());
        ok()
    }

    fn notify(&self, app: &App, event: &AppEventData) -> Void {
        if event.notify == NT_MOD_B_PING {
            let ch = match event.context_as::<char>() {
                Ok(s) => format!("{}", s.green()),
                _ => format!("{}", char::from_u32(0x2660).unwrap().bright_yellow())
            };
            print!("{ch}");
            unsafe {
                libc::write(libc::STDOUT_FILENO, null(), 0);
            };
        }

        let cmd = app.get::<CmdArgs>()?.get_cmd().unwrap_or_default();
        if event.notify == NT_APP_RUN && Self::meta().commands.contains(&cmd.as_str()) {
            event.handled.set(true);
            loop {
                app.notify(NT_MOD_B_PING, None, None, None)?;
                unsafe { libc::usleep(100_000) };
            }
        }

        ok()
    }
}

ld_meta!(LdMeta {
    lib: module_path!().into(),
    mods: [App::meta(MOD_B).clone()].into()
});
