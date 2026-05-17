#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate core;
extern crate alloc;

pub mod app;
pub mod app_c;
pub mod app_event;
pub mod app_hook;
pub mod app_module;
pub mod app_module_meta;
pub mod app_config;
pub mod modules;
pub mod pid_file;
pub mod macros;
pub mod prelude;

#[allow(unused_imports)]
use {crate::prelude::*, app_base::prelude::Once};

#[cfg(feature = "bind")]
ld_meta!({
    use crate::modules::*;
    LdMeta {
        lib: module_path!().into(),
        mods: [
            App::meta(MOD_CMD).clone(),
            App::meta(MOD_LD).clone(),
            App::meta(MOD_ENV).clone(),
            App::meta(MOD_LOG).clone(),
            App::meta(MOD_HELP).clone(),
            App::meta(MOD_CONFIG).clone(),
            #[cfg(feature = "async")]
            App::meta(MOD_ASYNC).clone()
        ]
        .into()
    }
});
