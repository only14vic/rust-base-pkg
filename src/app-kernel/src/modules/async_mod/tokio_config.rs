use {app_base::prelude::*, core::fmt::Display};

#[derive(Debug, Clone, ExtendFromIter)]
pub struct TokioConfig {
    pub threads: usize,
    pub blocking_threads: usize,
    pub blocking_threads_lifetime: u64,
    pub thread_name: String
}

impl Default for TokioConfig {
    fn default() -> Self {
        Self {
            threads: Env::get("TOKIO_THREADS")
                .map(|s| s.parse().expect("Invalid value"))
                .unwrap_or(1),
            blocking_threads: Env::get("TOKIO_BLOCKING_THREADS")
                .map(|s| s.parse().expect("Invalid value"))
                .unwrap_or(512),
            blocking_threads_lifetime: Env::get("TOKIO_BLOCKING_THREADS_LIFETIME")
                .map(|s| s.parse().expect("Invalid value"))
                .unwrap_or(60),
            thread_name: "tokio-worker".into()
        }
    }
}

impl IterConfig for TokioConfig {
    fn iter_config(&self) -> Vec<(&'static str, String)> {
        [
            ("tokio.threads", &self.threads as &dyn Display),
            ("tokio.blocking_threads", &self.blocking_threads),
            (
                "tokio.blocking_threads_lifetime", &self.blocking_threads_lifetime
            ),
            ("tokio.thread_name", &self.thread_name)
        ]
        .iter()
        .map(convert::tuple_string)
        .collect()
    }
}

impl TryFrom<&Di> for TokioConfig {
    type Error = Err;

    fn try_from(di: &Di) -> Result<Self, Self::Error> {
        let mut config = Self::default();
        let options = di.get::<ConfigOptions>()?;
        let args = di.get::<CmdArgs>()?;

        config.extend(options.filter("tokio"));
        config.extend(
            [
                ("threads", args.get_opt("tokio-threads")),
                ("blocking_threads", args.get_opt("tokio-blocking-threads")),
                (
                    "blocking_threads_lifetime",
                    args.get_opt("tokio-blocking-threads-lifetime")
                )
            ]
            .iter()
            .map(convert::tuple_option_string)
        );

        Ok(config)
    }
}
