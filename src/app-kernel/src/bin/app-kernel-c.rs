#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

extern crate alloc;
extern crate core;

// Don't add "use app_base" if used bindings_c.rs
//use app_base::prelude::*;
include!("../bindings_c.rs");

#[cfg(all(not(target_env = "musl"), not(feature = "std")))]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../app-base/src/no_std.rs"
));

#[cfg(not(feature = "std"))]
use {
    core::ffi::{c_char, c_int},
    libc_print::std_name::*
};

#[cfg(feature = "std")]
fn main() -> Result<(), Box<dyn Error>> {
    unsafe { app_main(&[MOD_CMD]) }
}

#[cfg(not(feature = "std"))]
#[unsafe(no_mangle)]
fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    unsafe {
        let res = app_main(&[MOD_CMD], argc, argv);

        if let Err(e) = res {
            eprintln!("Error: {e}");
            return libc::EXIT_FAILURE;
        }
    }

    libc::EXIT_SUCCESS
}
