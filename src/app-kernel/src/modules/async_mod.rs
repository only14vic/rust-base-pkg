use {
    app_base::prelude::*,
    core::{
        error::Error,
        fmt::{self, Display},
        sync::atomic::{AtomicBool, Ordering},
        time::Duration
    },
    futures::{
        FutureExt,
        channel::oneshot::{Receiver, Sender, channel},
        executor::block_on,
        future::{BoxFuture, Shared}
    },
    std::sync::Arc,
    tokio::{
        runtime::Runtime,
        select,
        signal::unix::{SignalKind, signal},
        spawn,
        sync::mpsc,
        task::{JoinHandle, LocalSet},
        time::sleep
    }
};
pub use {crate::*, async_init::*, async_runtime::*, tokio_config::*};

mod async_runtime;
mod async_init;
mod tokio_config;

pub static MOD_ASYNC: AppModule = AsyncMod::module;
pub const NT_ASYNC_START: &str = "Async::start";
pub const NT_ASYNC_STARTED: &str = "Async::started";
pub const NT_ASYNC_SPAWN: &str = "Async::spawn";

#[derive(Clone, Debug)]
pub struct AsyncStopMsg;
pub type AsyncStopSignal = Arc<Shared<Receiver<AsyncStopMsg>>>;
pub type AsyncStopSignals = Mutex<Vec<Sender<AsyncStopMsg>>>;
pub type AsyncRuntime = Arc<Runtime>;
pub type AsyncSwapChannel = Arc<mpsc::UnboundedSender<BoxFuture<'static, VoidSync>>>;

impl Display for AsyncStopMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for AsyncStopMsg {}

#[derive(Default)]
struct AsyncMod {
    is_started: AtomicBool
}

impl AsyncMod {
    fn start(&self, app: &App, with_block: bool) -> Void {
        if self.is_started.swap(true, Ordering::AcqRel) {
            return ok();
        }

        let tokio_config = app.get::<TokioConfig>()?;

        if tokio::runtime::Handle::try_current().is_err() {
            let rt = tokio_start(Some(&tokio_config))?;
            Box::leak(Box::new(rt.enter()));
            app.add(Arc::new(rt) as AsyncRuntime);
        }

        let stop = (**app.get::<AsyncStopSignal>()?).clone();
        let stop_signals = app.get::<AsyncStopSignals>()?;

        // Waits termination and drop stop signals
        let stop_clone = stop.clone();
        spawn(async move {
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            let mut sigint = signal(SignalKind::interrupt()).unwrap();
            let mut sigquit = signal(SignalKind::quit()).unwrap();

            select! {
                _ = sigterm.recv() => (),
                _ = sigint.recv() => (),
                _ = sigquit.recv() => (),
                _ = stop_clone => ()
            }

            *stop_signals.lock() = [].into();
        });

        // Spawn queue
        let (tx, mut rx) = mpsc::unbounded_channel::<BoxFuture<'static, VoidSync>>();
        app.add::<AsyncSwapChannel>(Arc::new(tx));
        let stop_clone = stop.clone();
        spawn(async move {
            select! {
                _ = async {
                    loop {
                        if let Some(fut) = rx.recv().await {
                            spawn(App::error_handler_async(fut, || module_path!()));
                        }
                    }
                } => (),
                _ = stop_clone => ()
            }
        });

        // for multi-threads
        if tokio::runtime::Handle::current().metrics().num_workers() > 1 {
            let stop_clone = stop.clone();
            spawn(async move {
                let fut = async {
                    let metrics = tokio::runtime::Handle::current().metrics();
                    let mut num_tasks_last = metrics.num_alive_tasks() as i64;
                    let timeout_min = Duration::from_nanos(1);
                    let timeout_max = Duration::from_millis(10);

                    loop {
                        let num_tasks_now = metrics.num_alive_tasks() as i64;

                        if num_tasks_now > 100 || num_tasks_now - num_tasks_last > 10 {
                            sleep(timeout_min).await;
                        } else {
                            sleep(timeout_max).await;
                        }

                        num_tasks_last = num_tasks_now;
                    }
                };
                select! {
                    _ = fut => (),
                    _ = stop_clone => {
                        log::debug!("Async stopped");
                    }
                };
            });
        }

        let app = app.get_weak::<App>()?.upgrade().unwrap();

        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                // Initialization async
                if let Some(async_init) = app.remove::<AsyncInit>() {
                    let async_init =
                        Arc::into_inner(async_init).ok_or("Arc<AsyncInit> is busy.")?;
                    async_init.init(&app).await?;
                }

                spawn(async move {
                    // Send event async started
                    let handle = tokio::runtime::Handle::current();
                    app.notify(NT_ASYNC_STARTED, None, Some(&handle), None)?;
                    ok() as VoidSync
                })
                .await??;

                // Block current thread
                if with_block {
                    LocalSet::new()
                        .run_until(async {
                            println!("Press Ctrl+C to cancel..");
                            stop.await.ok();
                            log::debug!("Async local stopped");
                        })
                        .await;
                }

                ok()
            })
        })
    }

    fn stop(&self, app: &App) -> Void {
        *app.get::<AsyncStopSignals>()?.lock() = [].into();

        std::thread::yield_now();
        std::thread::sleep(Duration::from_millis(1));

        if let Some(actix_system) = actix::System::try_current() {
            actix_system.stop();
        }

        if let Some(rt_tokio) = app
            .remove::<AsyncRuntime>()
            .and_then(Arc::into_inner)
            .and_then(Arc::into_inner)
        {
            rt_tokio.shutdown_timeout(Duration::from_millis(100));
        }

        self.is_started.store(false, Ordering::Release);

        ok()
    }
}

impl AppModuleExt for AsyncMod {
    app_module_meta!(AppModuleMeta {
        name: app_module_name!(MOD_ASYNC, no_mangle, #[cfg(feature = "bind")]),
        module: Self::module,
        depends: [MOD_CMD].into(),
        notifies: [
            NT_CONFIG_DISPLAY, NT_APP_SHUTDOWN, NT_ASYNC_START, NT_ASYNC_SPAWN
        ]
        .into(),
        sends: [NT_ASYNC_STARTED].into(),
        hooks: [&AppEvent::LOAD as &dyn AppHook].into(),
        commands: [].into()
    });

    fn init(&mut self, app: &App, _event: &AppEventData) -> Void {
        let (stop_tx, stop_rx) = channel::<AsyncStopMsg>();

        let stop_rx = stop_rx.shared();
        app.add(Arc::new(stop_rx.clone()) as AsyncStopSignal);

        app.add(AsyncStopSignals::default());
        app.get::<AsyncStopSignals>()?.lock().push(stop_tx);

        ok()
    }

    fn notify(&self, app: &App, event: &AppEventData) -> Void {
        if event.notify == NT_ASYNC_START {
            event.handled.set(true);
            let with_block = event.context_as::<bool>().unwrap_or(&false);
            self.start(app, *with_block)?;
        }

        if event.notify == NT_ASYNC_SPAWN {
            event.handled.set(true);
            let f = *event.context_as::<fn() -> BoxFuture<'static, VoidSync>>()?;
            event.set_result::<JoinHandle<VoidSync>>(spawn(f()))?;
        }

        if event.notify == NT_APP_SHUTDOWN {
            self.stop(app)?;
        }

        if event.notify == NT_CONFIG_DISPLAY {
            app.get::<ConfigDisplay>()?
                .write()
                .push(app.get::<TokioConfig>()?);
        }

        ok()
    }

    fn hook(
        &self,
        app: &App,
        event: &AppEventData,
        _hook_event: &AppEventData,
        is_pre_hook: bool
    ) -> Void {
        if event.event == AppEvent::LOAD
            && is_pre_hook == false
            && self.is_started.load(Ordering::Relaxed)
            && let Some(async_init) = app.remove::<AsyncInit>()
        {
            let async_init =
                Arc::into_inner(async_init).ok_or("Arc<AsyncInit> is busy.")?;
            block_on(async_init.init(app))?;
        }

        ok()
    }

    fn down(&mut self, app: &App, _event: &AppEventData) -> Void {
        self.stop(app)
    }
}
