#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

extern crate alloc;
extern crate core;

#[cfg(not(feature = "std"))]
use {
    core::ffi::{c_char, c_int},
    libc_print::std_name::*
};
#[cfg(feature = "std")]
use app_base::prelude::*;
use app_kernel::{app_cdylib_mod, prelude::App};

app_cdylib_mod!("app_kernel", MOD_CMD, app_kernel::modules::MOD_CMD);

#[cfg(feature = "std")]
fn main() -> Void {
    App::new(&[MOD_CMD()])?.run(None).void()
}

#[cfg(not(feature = "std"))]
#[unsafe(no_mangle)]
fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    let res = App::new(&[MOD_CMD()], argc, argv).and_then(|app| app.run(None));

    if let Err(e) = res {
        eprintln!("Error: {e}");
        return libc::EXIT_FAILURE;
    }

    libc::EXIT_SUCCESS
}
