mod common;

use {
    crate::common::MOD_EXAMPLE, app_base::prelude::*, app_kernel::prelude::*,
    core::ptr::slice_from_raw_parts
};

#[test]
fn test_app() -> Void {
    let app = App::new(&[MOD_EXAMPLE])?;
    let result = app.run(Some(&"Run context"))?;

    const TEST1: &str = "Hello";
    assert_eq!(
        &format!("{:p}", TEST1 as *const _ as *const usize),
        str_from_ptr(&TEST1)
    );

    const TEST2: AppModule = MOD_EXAMPLE;
    assert_eq!(
        &format!("{:p}", TEST2 as *const () as *const usize),
        str_from_ptr(&TEST2)
    );

    dbg!(str_from_ident!(TEST1));

    assert_eq!(
        Some(&"App result!"),
        result.and_then(|r| r.downcast::<&str>().ok()).as_deref()
    );

    ok()
}

#[macro_export]
macro_rules! str_from_ident {
    ($T:expr) => {{ $T }};
}

const fn str_from_ptr<T>(p: *const T) -> &'static str {
    const MAX: usize = 18;
    const ZERO: [u8; MAX] = [0u8; _];
    static mut BUF: [u8; MAX] = [0; _];

    unsafe {
        #[allow(static_mut_refs)]
        let buf = BUF.as_mut_slice();
        buf.copy_from_slice(&ZERO);

        let v = *p.cast::<usize>();
        let s = (&numtoa::numtoa_usize_str(v, 16, buf) as *const &str)
            .cast::<&mut str>()
            .cast_mut()
            .as_mut()
            .unwrap();
        s.make_ascii_lowercase();

        let mut c = 0;
        while buf[c] == 0 {
            c += 1;
        }

        buf.rotate_left(c - 2);
        buf[0] = b'0';
        buf[1] = b'x';

        str::from_utf8_unchecked(&*slice_from_raw_parts(buf.as_ptr(), MAX - c + 2))
    }
}
