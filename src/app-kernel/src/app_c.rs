#![cfg(feature = "bind")]

#[cfg(not(feature = "std"))]
use core::ffi::c_int;

use {
    crate::prelude::*,
    alloc::{boxed::Box, ffi::CString, rc::Rc, sync::Arc},
    app_base::prelude::{CmdArgs, TypedAny, Void, VoidExt},
    core::{
        error::Error,
        ffi::{CStr, c_char, c_uint, c_void},
        mem::transmute,
        ops::Not,
        slice::from_raw_parts
    }
};

pub type AppModule =
    extern "C" fn(*const App, *const AppEvent, *const c_void) -> *const c_void;

pub const unsafe fn into_app_mod(module: AppModule) -> crate::prelude::AppModule {
    unsafe { core::mem::transmute(module) }
}

#[unsafe(no_mangle)]
pub static NT_APP_RUN: &CStr = c"App::run";
#[unsafe(no_mangle)]
pub static NT_APP_SHUTDOWN: &CStr = c"App::shutdown";

#[unsafe(no_mangle)]
pub static ERR_APP: isize = -1;

#[allow(improper_ctypes)]
#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
unsafe extern "C" fn app_main(
    modules: &[super::app_module::AppModule],
    #[cfg(not(feature = "std"))] argc: c_int,
    #[cfg(not(feature = "std"))] argv: *const *const c_char
) -> Void {
    #[cfg(feature = "std")]
    return App::new(modules)?.run(None).void();

    #[cfg(not(feature = "std"))]
    return App::new(modules, argc, argv)?.run(None).void();
}

#[allow(unused_variables)]
#[unsafe(no_mangle)]
unsafe extern "C" fn app_new(
    modules: *const AppModule,
    count: c_uint,
    #[cfg(not(feature = "std"))] argc: c_int,
    #[cfg(not(feature = "std"))] argv: *const *const c_char
) -> *const App {
    let modules = unsafe { transmute(from_raw_parts(modules, count as usize)) };

    #[cfg(feature = "std")]
    let res = App::new(modules);

    #[cfg(not(feature = "std"))]
    let res = App::new(modules, argc, argv);

    let app = match res {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Error: {e}");
            return &ERR_APP as *const _ as *const App;
        }
    };

    Arc::into_raw(app)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_run(
    app: *const App,
    context: *const c_void
) -> *const *const c_void {
    let app = unsafe { &*app };
    let context = context.is_null().not().then_some(&context as &dyn TypedAny);

    let res = match app.run(context) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error: {e}");
            return &ERR_APP as *const _ as *const *const c_void;
        }
    };

    res.map(|r| Rc::into_raw(r).cast()).unwrap_or_default()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_notify(
    app: *const App,
    notify: *const c_char,
    sender: *const c_void,
    context: *const c_void
) -> *const *const c_void {
    let app = unsafe { &*app };
    let notify = unsafe { CStr::from_ptr(notify).to_str().unwrap() };
    let sender = sender.is_null().not().then(|| &sender as &dyn TypedAny);
    let context = context.is_null().not().then(|| &context as &dyn TypedAny);

    app.notify(notify, sender, context, None)
        .unwrap()
        .map(|r| Rc::into_raw(r).cast())
        .unwrap_or_default()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_free(app: *const App) {
    let _ = unsafe { Arc::from_raw(app) };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_set_default_cmd(cmd: *const c_char) {
    unsafe {
        let cmd = CStr::from_ptr(cmd).to_str().unwrap();
        CmdArgs::set_default_cmd(cmd);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_set_event_handled(event: *const AppEvent, handled: bool) {
    let event = unsafe { &*event.cast::<AppEventData>() };
    event.handled.set(handled);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_get_event_notify(event: *const AppEvent) -> *mut c_char {
    let event = unsafe { &*event.cast::<AppEventData>() };
    CString::new(event.notify).unwrap().into_raw()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_load_module(app: *const App, module: AppModule) {
    unsafe {
        let app = &*app;
        app.module_load(transmute(module)).unwrap();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_unload_module(app: *const App, module: AppModule) {
    unsafe {
        let app = &*app;
        app.module_unload(transmute(module)).unwrap();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_error(err: *const c_char) -> *const c_void {
    unsafe {
        let err: Box<dyn Error> = CStr::from_ptr(err).to_string_lossy().into();
        Box::into_raw(err).cast()
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_add_notify_handler(
    app: *const App,
    notify: *const c_char,
    module: AppModule
) -> bool {
    unsafe {
        let app = &*app;
        let notify = CStr::from_ptr(notify).to_str().unwrap();
        app.add_notify_handler(notify, transmute(module)).unwrap()
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_remove_notify_handler(
    app: *const App,
    notify: *const c_char,
    module: AppModule
) -> bool {
    unsafe {
        let app = &*app;
        let notify = CStr::from_ptr(notify).to_str().unwrap();
        app.remove_notify_handler(notify, transmute(module))
            .unwrap()
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_add_hook_handler(
    app: *const App,
    event: AppEvent,
    notify: *const c_char,
    module: AppModule
) -> bool {
    unsafe {
        let app = &*app;
        let notify = CStr::from_ptr(notify).to_str().unwrap();
        app.add_hook_handler(event, notify, transmute(module))
            .unwrap()
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn app_remove_hook_handler(
    app: *const App,
    event: AppEvent,
    notify: *const c_char,
    module: AppModule
) -> bool {
    unsafe {
        let app = &*app;
        let notify = CStr::from_ptr(notify).to_str().unwrap();
        app.remove_hook_handler(event, notify, transmute(module))
            .unwrap()
    }
}
