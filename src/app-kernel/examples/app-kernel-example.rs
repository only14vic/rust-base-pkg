#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

extern crate alloc;
extern crate core;

#[cfg(not(feature = "std"))]
use core::ffi::{c_char, c_int};

use {app_base::prelude::*, app_kernel::prelude::*};

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/common/mod.rs"));

#[cfg(feature = "std")]
fn main() -> Void {
    App::new(&[MOD_EXAMPLE])?.run(Some(&"Run context"))?;
    mem_stats();
    ok()
}

#[cfg(not(feature = "std"))]
#[unsafe(no_mangle)]
fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    let Ok(app) = App::new(&[MOD_EXAMPLE], argc, argv) else {
        return libc::EXIT_FAILURE;
    };

    if let Err(e) = app.run(Some(&"Run context")) {
        eprintln!("Error: {e}");
        return libc::EXIT_FAILURE;
    }

    mem_stats();

    libc::EXIT_SUCCESS
}
