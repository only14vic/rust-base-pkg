#[cfg(feature = "std")]
use std::panic::PanicHookInfo;

#[cfg(not(feature = "std"))]
use {core::ffi::c_char, core::ffi::c_int, core::panic::PanicInfo};
use {
    crate::prelude::*,
    alloc::{
        boxed::Box,
        format,
        rc::Rc,
        string::{String, ToString},
        sync::Arc,
        vec::Vec
    },
    app_base::prelude::*,
    core::{
        cell::RefCell,
        fmt::Display,
        mem::MaybeUninit,
        ops::{Deref, Not},
        ptr::{fn_addr_eq, null_mut},
        sync::atomic::{AtomicPtr, AtomicUsize, Ordering}
    },
    yansi::Paint
};

pub static APP: AtomicPtr<App> = AtomicPtr::new(null_mut());

/// cbindgen:ignore
pub const NT_APP_RUN: &str = "App::run";
/// cbindgen:ignore
pub const NT_APP_SHUTDOWN: &str = "App::shutdown";

#[derive(Debug, Default)]
struct AppData {
    modules: IndexSet<AppModule>,
    notifies: IndexMap<&'static str, Vec<AppModule>>,
    hooks: IndexMap<(AppEvent, &'static str), Vec<AppModule>>
}

#[derive(Debug)]
pub struct App {
    di: Arc<Di>,
    data: RwLock<AppData>,
    mod_pos: AtomicUsize
}

impl App {
    pub(crate) const MODS_STACK_LIMIT: usize = Self::mods_stack_limit();

    const fn mods_stack_limit() -> usize {
        if let Some(s) = option_env!("APP_BUILD_MODS_STACK_LIMIT")
            && let Ok(c) = usize::from_str_radix(s, 10)
        {
            c
        } else {
            20 // default
        }
    }

    pub fn new(
        modules: &[AppModule],
        #[cfg(not(feature = "std"))] argc: c_int,
        #[cfg(not(feature = "std"))] argv: *const *const c_char
    ) -> Ok<Arc<Self>> {
        #[cfg(feature = "std")]
        std::panic::set_hook(Box::new(Self::panic_handler));
        #[cfg(not(feature = "std"))]
        app_base::prelude::set_panic_handler(Box::new(Self::panic_handler));

        unsafe {
            let handler = Self::terminate as *const () as usize;
            libc::signal(libc::SIGINT, handler);
            libc::signal(libc::SIGTERM, handler);
            libc::signal(libc::SIGQUIT, handler);
        }

        let app = Self {
            di: Default::default(),
            data: Default::default(),
            mod_pos: AtomicUsize::new(0)
        };
        let di = app.di.clone();
        let app = Arc::new(app);
        di.add(Arc::downgrade(&app));

        APP.store((app.as_ref() as *const App).cast_mut(), Ordering::Release);

        #[cfg(feature = "std")]
        app.add(CmdArgsLine::default());
        #[cfg(not(feature = "std"))]
        app.add(CmdArgsLineParams::new(argc, argv));

        let modules: IndexSet<_> = modules.iter().cloned().collect();

        app.dispatch(&AppEventData {
            event: AppEvent::LOAD,
            sender: Some(app.as_ref()),
            context: Some(&modules),
            ..Default::default()
        })?;

        Ok(app)
    }

    pub fn di(&self) -> &Arc<Di> {
        &self.di
    }

    pub fn run(&self, context: Option<&dyn TypedAny>) -> Ok<Option<Rc<dyn TypedAny>>> {
        self.notify(NT_APP_RUN, Some(self), context, None)
    }

    #[inline(never)]
    fn terminate(signo: u32) -> ! {
        unsafe {
            libc::signal(libc::SIGINT, libc::SIG_DFL);
            libc::signal(libc::SIGTERM, libc::SIG_DFL);
            libc::signal(libc::SIGQUIT, libc::SIG_DFL);
        }

        log::debug!("App terminating on signal: {signo}");

        if let Some(app) = unsafe { APP.load(Ordering::Acquire).as_ref() } {
            app.shutdown(None);
        }

        #[cfg(feature = "std")]
        std::process::exit(libc::EXIT_SUCCESS);

        #[cfg(not(feature = "std"))]
        unsafe {
            libc::exit(libc::EXIT_SUCCESS)
        }
    }

    #[inline(never)]
    pub fn shutdown(&self, exit_code: Option<i32>) {
        log::debug!("App shutting down with exit code: {exit_code:?}");

        self.notify(
            NT_APP_SHUTDOWN,
            Some(self),
            exit_code.as_ref().map(|v| v as &dyn TypedAny),
            None
        )
        .map_err(|e| log::error!("{e}"))
        .ok();

        self.dispatch(&AppEventData {
            event: AppEvent::UNLOAD,
            sender: Some(self),
            ..Default::default()
        })
        .map_err(|e| log::error!("{e}"))
        .ok();

        Di::clear(&self.di);

        APP.store(null_mut(), Ordering::Release);

        if let Some(exit_code) = exit_code {
            #[cfg(feature = "std")]
            std::process::exit(exit_code);

            #[cfg(not(feature = "std"))]
            unsafe {
                libc::exit(exit_code)
            }
        }
    }

    #[inline]
    pub fn notify(
        &self,
        notify: &str,
        sender: Option<&dyn TypedAny>,
        context: Option<&dyn TypedAny>,
        result: Option<Rc<dyn TypedAny>>
    ) -> Ok<Option<Rc<dyn TypedAny>>> {
        let event = AppEventData {
            event: AppEvent::NOTIFY,
            notify,
            sender,
            context,
            result: RefCell::new(result),
            ..Default::default()
        };

        self.dispatch(&event)?;

        Ok(event.result.take())
    }

    #[inline]
    pub fn meta(module: AppModule) -> &'static AppModuleMeta<'static> {
        let event = AppEventData { event: AppEvent::META, ..Default::default() };

        module(None, &event, None).unwrap();

        *event
            .result_as::<_>()
            .unwrap_or_else(|e| {
                static META: Lazy<
                    RwLock<IndexMap<AppModule, &'static AppModuleMeta<'static>>>
                > = Lazy::new(|| RwLock::new(IndexMap::default()));

                if let Some(meta) = META.read().get(&module) {
                    Some(Rc::new(*meta))
                } else {
                    log::warn!("{e}");
                    let meta = Box::leak(Box::new(AppModuleMeta {
                        module,
                        name: String::leak(format!("{module:p}")),
                        depends: Default::default(),
                        sends: Default::default(),
                        notifies: Default::default(),
                        hooks: Default::default(),
                        commands: Default::default()
                    }));
                    META.write().insert(module, meta);
                    Some(Rc::new(meta))
                }
            })
            .unwrap()
    }

    #[inline]
    fn dispatch(&self, event: &AppEventData) -> Void {
        match event.event {
            AppEvent::LOAD => self.dispatch_load(event),
            AppEvent::UNLOAD => self.dispatch_unload(event),
            AppEvent::NOTIFY => self.dispatch_notify(event),
            AppEvent::HOOK => self.dispatch_hook(event),
            _ => unimplemented!("App::dispatch({})", event.event)
        }
    }

    #[inline(never)]
    fn dispatch_load(&self, event: &AppEventData) -> Void {
        let modules = event.context_as::<IndexSet<AppModule>>()?;
        Env::is_debug().then(|| {
            log::debug!(
                "Send {event:?}: {modules:?}",
                event = event.event,
                modules = modules.as_slice()
            )
        });

        for module in modules.iter() {
            Self::error_handler(
                || self.module_load(*module),
                || self.dispatch_log_target(event, Some(*module))
            )?;
        }

        ok()
    }

    #[inline(never)]
    fn dispatch_unload(&self, event: &AppEventData) -> Void {
        Env::is_debug().then(|| log::debug!("Send {event:?}", event = event.event));

        let modules = self.modules();

        for module in modules.iter().rev() {
            // no panic if error occured on unload module
            Self::error_handler(
                || self.module_unload(*module),
                || self.dispatch_log_target(event, Some(*module))
            )
            .ok();
        }

        ok()
    }

    #[inline(never)]
    fn dispatch_notify(&self, event: &AppEventData) -> Void {
        Env::is_debug().then(|| {
            log::debug!(
                "Send {event:?}: {notify:?}, {context:?}",
                event = event.event,
                notify = event.notify,
                context = event.context.and_then(|c| { unsafe { c.as_deref_ptr() } })
            )
        });

        let has_handlers = self
            .data
            .read()
            .notifies
            .get(event.notify)
            .is_some_and(|l| l.is_empty() == false);

        if has_handlers == false {
            return ok();
        }

        self.hooked_call(event, || {
            let mut modules_stack: MaybeUninit<[_; Self::MODS_STACK_LIMIT]> =
                MaybeUninit::uninit();
            let modules;

            let lock = self.data.read();

            if let Some(notifies) = lock.notifies.get(event.notify) {
                modules =
                    unsafe { &mut modules_stack.assume_init_mut()[0..notifies.len()] };
                modules.copy_from_slice(notifies.as_slice());
            } else {
                return ok();
            };

            drop(lock);

            for module in modules {
                if event.handled.get() {
                    break;
                }

                Self::error_handler(
                    || {
                        module(
                            Some(self),
                            event,
                            event.context.and_then(|c| unsafe { c.as_deref_ptr() })
                        )
                    },
                    || self.dispatch_log_target(event, Some(*module))
                )?;
            }

            ok()
        })
    }

    #[inline(never)]
    fn dispatch_hook(&self, event: &AppEventData) -> Void {
        let hook_context = event.context_as::<AppHookContext>()?;
        let origin_event = hook_context.origin_event;

        let mut modules_stack: MaybeUninit<[_; Self::MODS_STACK_LIMIT]> =
            MaybeUninit::uninit();
        let mut modules;
        let mut modules_count = 0;

        let lock = self.data.read();

        if let Some(hooks) = lock.hooks.get(&(origin_event.event, origin_event.notify)) {
            modules = unsafe { &mut modules_stack.assume_init_mut()[0..hooks.len()] };
            modules.copy_from_slice(hooks.as_slice());
            modules_count += hooks.len();
        }

        if let Some(hooks) = lock.hooks.get(&(origin_event.event, "")) {
            // Checks stack array overflow
            if modules_count + hooks.len() > Self::MODS_STACK_LIMIT {
                panic!(
                    "App::MODS_STACK_LIMIT is reached for hooks: {}",
                    Self::MODS_STACK_LIMIT
                );
            }

            modules = unsafe {
                &mut modules_stack.assume_init_mut()
                    [modules_count..modules_count + hooks.len()]
            };
            modules.copy_from_slice(hooks.as_slice());
            modules_count += hooks.len();
        }

        drop(lock);

        let modules = unsafe { &modules_stack.assume_init()[0..modules_count] };

        if modules.is_empty() {
            return ok();
        }

        Env::is_debug().then(|| {
            log::debug!(
                "Send {event:?} on {is_pre_hook} {origin_event:?}: {notify:?}, {context:?}",
                event = event.event,
                is_pre_hook = if hook_context.is_pre_hook.get() { "pre" } else { "post" },
                origin_event = origin_event.event,
                notify = event.notify,
                context = origin_event
                    .context
                    .and_then(|c| { unsafe { c.as_deref_ptr() } })
            )
        });

        for module in modules.iter() {
            // Hook may cancel event handling
            if event.handled.get() {
                break;
            }

            Self::error_handler(
                || {
                    module(
                        Some(self),
                        event,
                        origin_event
                            .context
                            .and_then(|c| unsafe { c.as_deref_ptr() })
                    )
                },
                || self.dispatch_log_target(event, Some(*module))
            )?;
        }

        ok()
    }

    #[inline(never)]
    fn dispatch_log_target(
        &self,
        event: &AppEventData,
        module: Option<AppModule>
    ) -> String {
        format!(
            concat!(
                module_path!(),
                "::dispatch(event: {}, notify: {:?}, sender: {:?}, context: {:?}, module: {:?})"
            ),
            event.event,
            event.notify,
            event.sender.map(|s| s.as_ptr()),
            event.context.map(|c| c.as_ptr()),
            module
        )
    }

    #[inline]
    fn hooked_call(&self, event: &AppEventData, callback: impl FnOnce() -> Void) -> Void {
        let hook_context = AppHookContext {
            is_pre_hook: true.into(),
            origin_event: unsafe {
                (event as *const _ as *const ())
                    .cast::<AppEventData<'static>>()
                    .as_ref_unchecked::<'static>()
            }
        };
        let hook_event = AppEventData {
            event: AppEvent::HOOK,
            notify: event.notify,
            sender: event.sender,
            context: Some(&hook_context),
            ..Default::default()
        };

        self.dispatch(&hook_event)?;

        // Hook may cancel event handling
        if hook_context.origin_event.handled.get() {
            return ok();
        }

        callback()?;

        hook_event.handled.set(false);
        hook_context.is_pre_hook.set(false);
        self.dispatch(&hook_event)?;

        ok()
    }

    /// Returns `clone` of module list.
    pub fn modules(&self) -> IndexSet<AppModule> {
        self.data.read().modules.clone()
    }

    #[inline(never)]
    pub fn module_load(&self, module: AppModule) -> Ok<bool> {
        let mut lock = self.data.write();

        if lock.modules.contains(&module) {
            return Ok(false);
        }

        let module_meta = Self::meta(module);
        if lock
            .modules
            .iter()
            .any(|m| Self::meta(*m).name == module_meta.name)
        {
            return Ok(false);
        }

        let len = lock.modules.len();
        let pos = self.mod_pos.load(Ordering::Acquire);
        let res = lock.modules.insert_before(pos.min(len), module).1;

        drop(lock);

        if res {
            Env::is_debug().then(|| log::debug!("Loading module: {module:p}"));
            let meta = App::meta(module);
            let event = AppEventData {
                event: AppEvent::LOAD,
                notify: meta.name,
                sender: Some(&module),
                context: Some(meta),
                ..Default::default()
            };

            self.hooked_call(&event, || {
                module(
                    Some(self),
                    &event,
                    event.context.and_then(|c| unsafe { c.as_deref_ptr() })
                )
            })?;

            self.mod_pos.fetch_add(1, Ordering::Release);
        }

        Ok(res)
    }

    #[inline(never)]
    pub fn module_unload(&self, module: AppModule) -> Ok<bool> {
        let mut lock = self.data.write();

        if lock.modules.contains(&module) == false {
            return Ok(false);
        }

        let res = lock.modules.shift_remove(&module);

        if res {
            Env::is_debug().then(|| log::debug!("Unloading module: {module:p}"));

            self.mod_pos.store(lock.modules.len(), Ordering::Release);

            drop(lock);

            let meta = App::meta(module);
            let event = AppEventData {
                event: AppEvent::UNLOAD,
                notify: meta.name,
                sender: Some(&module),
                context: Some(meta),
                ..Default::default()
            };

            self.hooked_call(&event, || {
                module(
                    Some(self),
                    &event,
                    event.context.and_then(|c| unsafe { c.as_deref_ptr() })
                )?;

                let mut lock = self.data.write();

                lock.notifies.iter_mut().for_each(|(.., notifies)| {
                    notifies.retain(|m| fn_addr_eq(*m, module).not());
                });

                lock.hooks.iter_mut().for_each(|(.., hooks)| {
                    hooks.retain(|m| fn_addr_eq(*m, module).not());
                });

                ok()
            })?;
        }

        Ok(res)
    }

    #[inline(never)]
    pub fn add_hook_handler(
        &self,
        event: AppEvent,
        notify: &'static str,
        module: AppModule
    ) -> Ok<bool> {
        if event == AppEvent::HOOK {
            return Err("Forbidden register hook on event AppEvent::HOOK")?;
        }

        let mut lock = self.data.write();
        let key = (event, &*String::leak(notify.to_string()));

        if lock.hooks.contains_key(&key) == false {
            lock.hooks.insert(key, Default::default());
        }

        let hooks = lock.hooks.get_mut(&key).unwrap();
        let count = hooks.len();

        if hooks.contains(&module) == false {
            if hooks.len() >= Self::MODS_STACK_LIMIT {
                panic!(
                    "App::MODS_STACK_LIMIT is reached for hooks: {}",
                    Self::MODS_STACK_LIMIT
                );
            }

            hooks.push(module);
        }

        Ok(count != hooks.len())
    }

    #[inline(never)]
    pub fn remove_hook_handler(
        &self,
        event: AppEvent,
        notify: &'static str,
        module: AppModule
    ) -> Ok<bool> {
        let mut lock = self.data.write();
        let key = (event, notify);

        if lock.hooks.contains_key(&key) == false {
            return Ok(false);
        }

        let hooks = lock.hooks.get_mut(&key).unwrap();
        let count = hooks.len();

        if hooks.contains(&module) {
            hooks.retain(|m| fn_addr_eq(*m, module).not());
        }

        Ok(count != hooks.len())
    }

    #[inline(never)]
    pub fn add_notify_handler(
        &self,
        notify: &'static str,
        module: AppModule
    ) -> Ok<bool> {
        let mut lock = self.data.write();

        if lock.notifies.contains_key(notify) == false {
            lock.notifies
                .insert(&*String::leak(notify.to_string()), Default::default());
        }

        let notifies = lock.notifies.get_mut(notify).unwrap();
        let count = notifies.len();

        if notifies.contains(&module) == false {
            if notifies.len() >= Self::MODS_STACK_LIMIT {
                panic!(
                    "App::MODS_STACK_LIMIT is reached for notifies: {}",
                    Self::MODS_STACK_LIMIT
                );
            }

            notifies.push(module);
        }

        Ok(count != notifies.len())
    }

    #[inline(never)]
    pub fn remove_notify_handler(
        &self,
        notify: &'static str,
        module: AppModule
    ) -> Ok<bool> {
        let mut lock = self.data.write();

        if lock.notifies.contains_key(notify) == false {
            return Ok(false);
        }

        let notifies = lock.notifies.get_mut(notify).unwrap();
        let count = notifies.len();

        if notifies.contains(&module) {
            notifies.retain(|m| fn_addr_eq(*m, module).not());
        }

        Ok(count != notifies.len())
    }

    fn panic_handler(
        #[cfg(feature = "std")] info: &PanicHookInfo,
        #[cfg(not(feature = "std"))] info: &PanicInfo
    ) {
        eprintln!("PANIC: {info}");
        log::error!("{info}");
    }

    #[inline]
    pub fn error_handler<T, E: Display, S: AsRef<str>>(
        callback: impl FnOnce() -> Result<T, E>,
        log_target: impl FnOnce() -> S
    ) -> Result<T, E> {
        callback().inspect_err(|e| {
            let target = log_target();
            let target = target.as_ref();
            Env::is_debug().then(|| {
                eprintln!("{}: {target}: {e}", "ERROR".bright_red());
            });
            log::error!(target: target, "{e}");
        })
    }

    #[inline]
    pub async fn error_handler_async<T, E: Display, S: AsRef<str>>(
        callback: impl Future<Output = Result<T, E>>,
        log_target: impl FnOnce() -> S
    ) -> Result<T, E> {
        let res = callback.await;
        if res.is_err() {
            Self::error_handler(|| res, log_target)
        } else {
            res
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.shutdown(None);
    }
}

impl Deref for App {
    type Target = Di;

    fn deref(&self) -> &Self::Target {
        &self.di
    }
}
