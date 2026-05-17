use {crate::common::MOD_ASYNC_EXAMPLE, app_base::prelude::*, app_kernel::prelude::*};

mod common;

#[test]
fn test_async_app() -> Void {
    App::new(&[MOD_ASYNC_EXAMPLE])?.run(None).void()
}
