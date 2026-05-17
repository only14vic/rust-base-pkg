use {
    crate::prelude::*,
    alloc::{format, sync::Arc},
    app_base::prelude::*,
    core::{ffi::c_void, ptr::NonNull}
};

/// cbindgen:ignore
pub type AppModule = fn(Option<&App>, &AppEventData, Option<NonNull<()>>) -> Void;

/// cbindgen:ignore
pub type AppModuleC =
    extern "C" fn(*const App, *const AppEvent, *const *const c_void) -> *const c_void;

pub trait AppModuleExt: Default + Send + Sync + 'static {
    #[inline(never)]
    fn module(
        app: Option<&App>,
        event: &AppEventData,
        context: Option<NonNull<()>>
    ) -> Void {
        if event.event == AppEvent::META {
            return event.set_result(Self::meta());
        }

        let app = app.ok_or("Argument 'app' is empty in AppModuleExt::handle()")?;

        let module_type = type_name_simple::<Self>();

        if event.event == AppEvent::LOAD {
            app.add_if_not_exist(Arc::new(Self::new(app)?));
        }

        let Ok(module) = app.get::<Arc<Self>>() else {
            return Err(format!("Module '{module_type}' does not found in App"))?;
        };

        match event.event {
            AppEvent::LOAD => {
                if let Ok(logger) = app.get::<Arc<&mut Logger>>() {
                    logger.init()?;
                }

                let meta = Self::meta();

                for notify in meta.notifies.iter() {
                    app.add_notify_handler(notify, Self::module)?;
                }

                for hook in meta.hooks.iter() {
                    app.add_hook_handler(hook.event(), hook.notify(), Self::module)?;
                }

                Env::is_debug().then(|| log::trace!("{module_type}::init({context:?})"));
                unsafe { module.as_ref().try_mut_unchecked() }?.init(app, event)?;

                for module in meta.depends.iter() {
                    app.module_load(*module)?;
                }

                Env::is_debug().then(|| log::trace!("{module_type}::boot({context:?})"));
                unsafe { module.as_ref().try_mut_unchecked() }?.boot(app, event)
            },
            AppEvent::UNLOAD => {
                Env::is_debug().then(|| log::trace!("{module_type}::down({context:?})"));
                app.remove::<Arc<Self>>();
                unsafe { module.as_ref().try_mut_unchecked() }?.down(app, event)
            },
            AppEvent::NOTIFY => {
                Env::is_debug().then(|| {
                    log::trace!("{module_type}::notify({:?}, {context:?})", event.notify)
                });
                module.notify(app, event)
            },
            AppEvent::HOOK => {
                let hook_context = event.context_as::<AppHookContext>()?;
                Env::is_debug().then(|| {
                    log::trace!(
                        "{module_type}::hook({} {}: {:?}, {context:?})",
                        if hook_context.is_pre_hook.get() { "pre" } else { "post" },
                        hook_context.origin_event.event,
                        hook_context.origin_event.notify
                    )
                });
                module.hook(
                    app,
                    hook_context.origin_event,
                    event,
                    hook_context.is_pre_hook.get()
                )
            },
            _ => unimplemented!("AppModuleExt::handle({})", event.event)
        }
    }

    fn meta() -> &'static AppModuleMeta<'static>;

    fn new(_app: &App) -> Ok<Self> {
        Ok(Self::default())
    }

    fn init(&mut self, _app: &App, _event: &AppEventData) -> Void {
        ok()
    }

    fn boot(&mut self, _app: &App, _event: &AppEventData) -> Void {
        ok()
    }

    fn notify(&self, _app: &App, _event: &AppEventData) -> Void {
        ok()
    }

    fn hook(
        &self,
        _app: &App,
        _event: &AppEventData,
        _hook_event: &AppEventData,
        _is_pre_hook: bool
    ) -> Void {
        ok()
    }

    fn down(&mut self, _app: &App, _event: &AppEventData) -> Void {
        ok()
    }
}
