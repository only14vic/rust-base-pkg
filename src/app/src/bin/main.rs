#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

extern crate alloc;
extern crate core;

#[cfg(not(feature = "std"))]
use core::ffi::{c_char, c_int};

#[allow(unused_imports)]
use {
    app_base::prelude::*,
    app_kernel::{
        app_cdylib_mod,
        prelude::{App, AppModule}
    }
};

app_cdylib_mod!("app", MOD_MAIN, app::MOD_MAIN);

#[cfg(prefer_dynamic)]
#[link(name = "app")]
unsafe extern "C" {
    #[allow(improper_ctypes)]
    fn app_main(modules: &[AppModule]) -> Void;
}

#[cfg(feature = "std")]
fn main() -> Void {
    App::new(&[MOD_MAIN()])?.run(None).void()
}

#[cfg(not(feature = "std"))]
#[unsafe(no_mangle)]
fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    let res = unsafe { app_main(&[MOD_MAIN()], argc, argv) };

    if let Err(e) = res {
        eprintln!("Error: {e}");
        return libc::EXIT_FAILURE;
    }

    libc::EXIT_SUCCESS
}
