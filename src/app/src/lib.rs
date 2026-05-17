#![cfg_attr(not(feature = "std"), no_std)]

#[allow(unused_imports)]
#[macro_use]
extern crate core;
extern crate alloc;

use app_base::prelude::*;

mod main_config;
mod main_mod;

pub use {main_config::*, main_mod::*};
