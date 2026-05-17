use {
    alloc::{
        boxed::Box,
        format,
        string::{String, ToString},
        vec::Vec
    },
    app_base::prelude::*,
    core::{fmt, fmt::Display, ops::Deref}
};

#[derive(Debug, Default)]
pub struct CmdAlias(Vec<String>);

impl Deref for CmdAlias {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for CmdAlias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.join(","))
    }
}

impl From<Option<&str>> for CmdAlias {
    fn from(value: Option<&str>) -> Self {
        Self(
            value
                .unwrap_or_default()
                .split(&[',', ' '])
                .filter(|s| s.is_empty() == false)
                .map(str::to_string)
                .collect()
        )
    }
}

#[derive(Debug, Default, ExtendFromIter)]
pub struct CmdConfig {
    pub default: Option<Box<str>>,
    pub bins: IndexMap<String, CmdAlias>
}

impl IterConfig for CmdConfig {
    fn iter_config(&self) -> Vec<(&'static str, String)> {
        [
            [("cmd.default", &self.default.as_ref().map_or("", |s| &**s))]
                .iter()
                .map(convert::tuple_string)
                .collect::<Vec<_>>(),
            self.bins
                .iter()
                .map(|(n, v)| ("cmd.bins", format!("{n}={v}",)))
                .collect()
        ]
        .concat()
    }
}

impl TryFrom<&Di> for CmdConfig {
    type Error = Err;

    fn try_from(di: &Di) -> Result<Self, Self::Error> {
        let mut config = Self::default();
        let options = di.get::<ConfigOptions>()?;

        config.extend(options.filter("cmd"));

        Ok(config)
    }
}
