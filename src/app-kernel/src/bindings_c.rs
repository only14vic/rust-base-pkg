// # Use this module like this:
// include!("bindings_c.rs");

use core::error::Error;

#[cfg(not(target_env = "musl"))]
include!("bindings_gen.rs");

#[cfg(not(target_env = "musl"))]
#[link(name = "app_kernel")]
unsafe extern "C" {
    #[allow(improper_ctypes)]
    #[cfg(feature = "std")]
    fn app_main(modules: &[AppModule]) -> Result<(), Box<dyn Error>>;

    #[allow(improper_ctypes)]
    #[cfg(not(feature = "std"))]
    fn app_main(
        modules: &[AppModule],
        argc: c_int,
        argv: *const *const c_char
    ) -> Result<(), Box<dyn Error>>;
}
