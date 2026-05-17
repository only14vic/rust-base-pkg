use {
    crate::prelude::*,
    alloc::{
        borrow::ToOwned,
        boxed::Box,
        fmt::Debug,
        format,
        string::{String, ToString},
        vec::Vec
    },
    app_base::prelude::*,
    core::{fmt::Display, mem::ManuallyDrop}
};

#[derive(Debug)]
pub struct LdItem<'a> {
    pub meta: LdMeta<'a>,
    pub lib: Option<libloading::Library>
}

#[allow(clippy::type_complexity)]
#[derive(Debug, ExtendFromIter)]
pub struct LdConfig {
    pub no_load: bool,
    pub lib_dir: Box<str>,
    pub load_libs: IndexSet<Box<str>>,
    pub load_mods: IndexSet<Box<str>>,
    /// [lib_name => LdItem]
    #[extend(skip)]
    pub(super) loaded_libs: ManuallyDrop<Mutex<IndexMap<Box<str>, LdItem<'static>>>>,
    /// [mod_name => (AppModule, lib_name)]
    #[extend(skip)]
    pub(super) loaded_mods: Mutex<IndexMap<Box<str>, (AppModule, Box<str>)>>
}

impl LdConfig {
    pub const OPT_PATH: &str = "ld-path";
    pub const OPT_LOAD_LIB: &str = "L:ld-lib";
    pub const OPT_LOAD_MOD: &str = "M:ld-mod";
    pub const OPT_NO_LOAD: &str = "N:ld-no";

    pub fn get_lib_path(&self, lib_name: &str) -> Box<str> {
        let mut path = lib_name.to_owned();

        if path.starts_with(['/', '.']) == false && path.starts_with("lib") == false {
            path.insert_str(0, "lib");
        }

        if path.ends_with(".so") == false {
            path.push_str(".so");
        }

        if lib_name.starts_with(['/', '.']) == false {
            path.insert(0, '/');
            path.insert_str(0, &self.lib_dir);
        };

        path.into()
    }

    pub fn get_lib_name(&self, path: &str) -> Box<str> {
        path.get(
            path.rfind("/lib").map(|p| p + 4).unwrap_or(0)
                ..path.rfind('.').unwrap_or(path.len())
        )
        .unwrap()
        .into()
    }

    pub fn get_module_name(&self, module_name: &str) -> Box<str> {
        if module_name.starts_with("MOD_") {
            module_name.to_ascii_uppercase().into()
        } else {
            format!("MOD_{}", module_name.to_ascii_uppercase()).into()
        }
    }

    pub fn find_lib_by_module(&self, module_name: &str) -> Option<Box<str>> {
        self.loaded_libs.lock().iter().find_map(|(path, item)| {
            item.meta
                .mods
                .iter()
                .any(|meta| meta.name == module_name)
                .then(|| path.clone())
        })
    }

    pub fn find_module_by_command(&self, cmd: &str) -> Option<&str> {
        self.loaded_libs.lock().iter().find_map(|(_, item)| {
            item.meta
                .mods
                .iter()
                .find_map(|meta| meta.commands.contains(&cmd).then_some(meta.name))
        })
    }

    pub fn find_module_by_name(&self, module_name: &str) -> Option<AppModule> {
        self.loaded_libs.lock().iter().find_map(|(_, item)| {
            item.meta
                .mods
                .iter()
                .find_map(|meta| (meta.name == module_name).then_some(meta.module))
        })
    }

    pub fn find_meta_by_name<'a>(
        &'a self,
        module_name: &str
    ) -> Option<&'a AppModuleMeta<'a>> {
        self.loaded_libs.lock().iter().find_map(|(_, item)| {
            item.meta.mods.iter().find_map(|meta| {
                (meta.name == module_name).then_some(unsafe { &*(meta as *const _) })
            })
        })
    }
}

impl Default for LdConfig {
    fn default() -> Self {
        Self {
            no_load: Env::get("LD_NO")
                .map(|v| ["1", "on", "true"].contains(&&*v))
                .unwrap_or_default(),
            lib_dir: Env::get("LD_PATH")
                .unwrap_or_default()
                .to_string()
                .into_boxed_str(),
            load_libs: Env::get("LD_LOAD_LIB")
                .map(|v| {
                    v.split(&[',', ' '])
                        .filter(|s| s.is_empty() == false)
                        .map(|s| s.to_string().into_boxed_str())
                        .collect()
                })
                .unwrap_or_default(),
            load_mods: Env::get("LD_LOAD_MOD")
                .map(|v| {
                    v.split(&[',', ' '])
                        .filter(|s| s.is_empty() == false)
                        .map(|s| s.to_string().into_boxed_str())
                        .collect()
                })
                .unwrap_or_default(),
            loaded_libs: Default::default(),
            loaded_mods: Default::default()
        }
    }
}

impl IterConfig for LdConfig {
    fn iter_config(&self) -> Vec<(&'static str, String)> {
        [
            [
                ("ld.path", &self.lib_dir as &dyn Display),
                ("ld.no_load", &self.no_load),
                (
                    "ld.load_libs",
                    &self
                        .load_libs
                        .iter()
                        .map(Box::as_ref)
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                (
                    "ld.load_mods",
                    &self
                        .load_mods
                        .iter()
                        .map(Box::as_ref)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            ]
            .iter()
            .map(convert::tuple_string)
            .collect::<Vec<_>>(),
            self.loaded_libs
                .lock()
                .keys()
                .map(|f| ("ld.loaded_libs", f.to_string()))
                .collect(),
            self.loaded_mods
                .lock()
                .keys()
                .map(|f| ("ld.loaded_mods", f.to_string()))
                .collect()
        ]
        .concat()
    }
}

impl TryFrom<&Di> for LdConfig {
    type Error = Err;

    fn try_from(di: &Di) -> Result<Self, Self::Error> {
        let mut config = Self::default();
        let dirs = di.get::<Dirs>()?;
        let args = di.get::<CmdArgs>()?;

        config.extend(di.get::<ConfigOptions>()?.filter("ld"));
        config.extend(
            [
                ("lib_dir", args.get_opt(Self::OPT_PATH)),
                ("no_load", args.get_opt(Self::OPT_NO_LOAD))
            ]
            .iter()
            .map(convert::tuple_option_str)
        );
        config.extend(
            [
                (
                    "load_libs",
                    args.get_opt_list(Self::OPT_LOAD_LIB).map(|v| v.join(","))
                ),
                (
                    "load_mods",
                    args.get_opt_list(Self::OPT_LOAD_MOD).map(|v| v.join(","))
                )
            ]
            .iter()
            .map(convert::tuple_option_string)
        );

        if config.lib_dir.is_empty() {
            config.lib_dir = dirs.lib.to_string().into();
        }

        Ok(config)
    }
}
