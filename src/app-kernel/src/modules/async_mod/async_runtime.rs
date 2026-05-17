use {
    crate::prelude::*,
    app_base::prelude::*,
    core::time::Duration,
    std::{future::Future, io::Result as IoResult, sync::LazyLock},
    tokio::runtime::Runtime
};

pub fn tokio_start(config: Option<&TokioConfig>) -> IoResult<Runtime> {
    static DEFAULT_CONFIG: LazyLock<TokioConfig> = LazyLock::new(Default::default);

    let config = config.unwrap_or(&DEFAULT_CONFIG);

    if config.threads == 0 {
        return tokio::runtime::Builder::new_current_thread()
            .max_blocking_threads(config.blocking_threads)
            .thread_keep_alive(Duration::from_secs(config.blocking_threads_lifetime))
            .thread_name(&config.thread_name)
            .enable_all()
            .build();
    }

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.threads)
        .max_blocking_threads(config.blocking_threads)
        .thread_keep_alive(Duration::from_secs(config.blocking_threads_lifetime))
        .thread_name(&config.thread_name)
        .enable_all()
        .build()
}

pub fn tokio_start_custom(
    app: &App,
    thread_name: &str,
    threads: usize
) -> Ok<&'static mut Runtime> {
    let tokio_config = TokioConfig {
        threads,
        thread_name: thread_name.into(),
        ..*app.get::<TokioConfig>()?
    };

    let rt = tokio_start(Some(&tokio_config))?;
    Box::leak(Box::new(rt.enter()));

    Ok(Box::leak(Box::new(rt)))
}

pub fn actix_with_tokio_start<T>(
    config: Option<&TokioConfig>,
    fut: impl Future<Output = T>
) -> IoResult<T> {
    let rt = tokio_start(config)?;
    let res = actix::System::with_tokio_rt(|| rt).block_on(fut);

    Ok(res)
}
