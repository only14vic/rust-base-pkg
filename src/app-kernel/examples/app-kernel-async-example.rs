include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/common/mod.rs"));

use {app_base::prelude::*, app_kernel::prelude::*};

fn main() -> Void {
    App::new(&[MOD_ASYNC_EXAMPLE])?.run(None).void()
}
