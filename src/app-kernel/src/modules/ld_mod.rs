use {crate::prelude::*, alloc::format, app_base::prelude::*};

mod ld_config;
mod ld_control;
mod ld_meta;

pub use {ld_config::*, ld_control::*, ld_meta::*};

pub static MOD_LD: AppModule = LdMod::module;

pub(super) const NT_LD_LOAD_MODULE: &str = "Ld::load_module";
pub(super) const NT_LD_LOAD_LIBRARY: &str = "Ld::load_library";
pub(super) const NT_LD_CLOSE_LIBRARY: &str = "Ld::close_library";
pub(super) const FN_LD_META: &str = "__ld_meta";

#[derive(Default)]
struct LdMod;

impl LdMod {
    fn load_module(&self, app: &App, module_name: &str) -> Ok<AppModule> {
        let config = app.get::<LdConfig>()?;

        let module_name_orig = module_name;
        let module_name = config.get_module_name(module_name);

        if let Some((module, _)) = config.loaded_mods.lock().get(&module_name) {
            return Ok(*module);
        }

        let Some(path) = config.find_lib_by_module(&module_name) else {
            return Err(format!("Library not found for module: {module_name_orig}"))?;
        };

        self.load_library(app, &path, false)?;

        Env::is_debug()
            .then(|| log::debug!("Loading symbol to load module: {module_name_orig}"));

        let loaded = config.loaded_libs.lock();
        let item = loaded.get(&path).unwrap();
        let lib = item
            .lib
            .as_ref()
            .ok_or_else(|| format!("Library '{path}' not opened"))?;

        let symbol: libloading::Symbol<*const AppModule> = unsafe {
            lib.get(&*module_name).map_err(|e| {
                format!("Could not find symbol of module '{module_name}': {e}")
            })?
        };

        let module: AppModule = unsafe { *(*symbol).cast() };

        drop(loaded);

        app.module_load(module)?;

        config
            .loaded_mods
            .lock()
            .insert(module_name, (module, config.get_lib_name(&path)));

        Ok(module)
    }

    fn load_library(&self, app: &App, lib_name: &str, only_meta: bool) -> Void {
        let config = app.get::<LdConfig>()?;
        let path = config.get_lib_path(lib_name);
        let mut loaded = config.loaded_libs.lock();

        if loaded.contains_key(&path) == false {
            Env::is_debug().then(|| log::debug!("Loading library: {path}"));

            if Dirs::exists(&path) == false {
                return Err(format!("Library not found: {path}"))?;
            }

            unsafe {
                let lib = libloading::Library::new(&*path)?;
                let symbol: libloading::Symbol<
                    unsafe extern "C" fn() -> *const LdMeta<'static>
                > = lib.get(FN_LD_META).map_err(|e| {
                    format!("Function {FN_LD_META} not found in library {path}: {e}")
                })?;

                let meta = (&*symbol()).clone();

                let lib = if only_meta {
                    lib.close()?;
                    None
                } else {
                    Some(lib)
                };

                Env::is_debug().then(|| log::trace!("Loaded library: {meta:?}"));

                loaded.insert(path.clone(), LdItem { meta, lib });
            }
        }

        if only_meta == false {
            let item = loaded.get_mut(&path).unwrap();
            if item.lib.is_none() {
                Env::is_debug().then(|| log::debug!("Opening library: {path}"));
                item.lib = Some(unsafe { libloading::Library::new(&*path)? });
            }
        }

        ok()
    }

    fn close_library(&self, app: &App, lib_name: &str) -> Void {
        let config = app.get::<LdConfig>()?;
        let path = config.get_lib_path(lib_name);
        let mut loaded_libs = config.loaded_libs.lock();

        let Some(lib) = loaded_libs.get_mut(&path).and_then(|item| item.lib.take())
        else {
            return ok();
        };

        Env::is_debug().then(|| log::debug!("Closing library: {path}"));

        drop(loaded_libs);

        let modules = config.loaded_mods.lock().clone();

        for (module_name, (module, module_lib_name)) in modules {
            if &*module_lib_name != lib_name {
                continue;
            }

            config.loaded_mods.lock().swap_remove(&module_name);

            app.module_unload(module)?;
        }

        lib.close()?;

        Env::is_debug().then(|| log::debug!("Library closed: {lib_name}"));

        ok()
    }
}

impl AppModuleExt for LdMod {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_LD, no_mangle, #[cfg(feature = "bind")]),
        module: Self::module,
        depends: [MOD_CMD].into(),
        sends: [NT_LD_LOAD_MODULE, NT_LD_LOAD_LIBRARY, NT_LD_CLOSE_LIBRARY].into(),
        notifies: [
            NT_LD_LOAD_MODULE, NT_LD_LOAD_LIBRARY, NT_LD_CLOSE_LIBRARY, NT_CONFIG_DISPLAY
        ]
        .into(),
        hooks: [].into(),
        commands: [].into()
    });

    fn init(&mut self, app: &App, _event: &AppEventData) -> Void {
        app.get::<CmdArgsLine>()?
            .write()
            .add_flags(&[LdConfig::OPT_NO_LOAD])?
            .add_alias(&[LdConfig::OPT_LOAD_LIB, LdConfig::OPT_LOAD_MOD])?;

        ok()
    }

    fn boot(&mut self, app: &App, _event: &AppEventData) -> Void {
        let config = app.get::<LdConfig>()?;
        let args = app.get::<CmdArgs>()?;

        if config.no_load {
            return ok();
        }

        for lib_name in config.load_libs.iter() {
            if let Err(e) = self.load_library(app, lib_name, true) {
                log::warn!("{e}");
            }
        }

        for module_name in config.load_mods.iter() {
            if let Err(e) = self.load_module(app, module_name) {
                log::warn!("{e}");
            }
        }

        let cmd = args.get_cmd().unwrap_or_default();

        if cmd.is_empty() == false
            && let Some(module_name) = config.find_module_by_command(&cmd)
        {
            self.load_module(app, module_name)?;
        }

        ok()
    }

    fn notify(&self, app: &App, event: &AppEventData) -> Void {
        match event.notify {
            NT_CONFIG_DISPLAY => {
                app.get::<ConfigDisplay>()?
                    .write()
                    .push(app.get::<LdConfig>()?);
            },
            NT_LD_LOAD_MODULE => {
                let module_name = event.context_as::<&str>()?;
                let module = self.load_module(app, module_name)?;
                event.set_result(module)?;
            },
            NT_LD_LOAD_LIBRARY => {
                let lib_name = event.context_as::<&str>()?;
                self.load_library(app, lib_name, true)?;
            },
            NT_LD_CLOSE_LIBRARY => {
                let lib_name = event.context_as::<&str>()?;
                self.close_library(app, lib_name)?;
            },
            _ => ()
        }

        ok()
    }
}
