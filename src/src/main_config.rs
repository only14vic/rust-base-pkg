use {
    alloc::{
        boxed::Box,
        fmt::Debug,
        string::{String, ToString},
        vec::Vec
    },
    app_base::prelude::*,
    core::fmt::Display
};

#[derive(Debug, ExtendFromIter)]
pub struct MainConfig {
    pub foo: Option<Box<str>>
}

impl Default for MainConfig {
    fn default() -> Self {
        Self {
            foo: Env::get("FOO")
                .as_deref()
                .or(Some("World"))
                .map(|v| v.into())
        }
    }
}

impl IterConfig for MainConfig {
    fn iter_config(&self) -> Vec<(&'static str, String)> {
        [
            ("app.features", &env!("BUILD_FEATURES") as &dyn Display),
            ("main.foo", &self.foo.as_deref().unwrap_or_default())
        ]
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect()
    }
}

impl TryFrom<&Di> for MainConfig {
    type Error = Err;

    fn try_from(di: &Di) -> Result<Self, Self::Error> {
        let mut config = Self::default();
        let options = di.get::<ConfigOptions>()?;
        let args = di.get::<CmdArgs>()?;

        config.extend(options.filter("main"));
        config.extend(
            [("foo", args.get_opt("foo"))]
                .iter()
                .map(convert::tuple_option_string)
        );

        Ok(config)
    }
}
