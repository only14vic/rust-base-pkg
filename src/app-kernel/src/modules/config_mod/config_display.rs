use {
    alloc::{format, string::ToString, sync::Arc, vec::Vec},
    app_base::prelude::*,
    core::{fmt, fmt::Display, ops::Deref}
};

#[derive(Default)]
pub struct ConfigDisplay(RwLock<Vec<Arc<dyn IterConfig>>>);

impl TryFrom<&Di> for ConfigDisplay {
    type Error = Err;

    fn try_from(_di: &Di) -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
}

impl Deref for ConfigDisplay {
    type Target = RwLock<Vec<Arc<dyn IterConfig>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for ConfigDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list: Vec<_> = self
            .read()
            .iter()
            .map(|c| c.iter_config())
            .collect::<Vec<_>>()
            .concat()
            .into_iter()
            .filter_map(|(k, v)| {
                k.is_empty()
                    .then_some(v.to_string())
                    .or(Some(format!("{k}={v}")))
            })
            .collect();

        list.sort();
        list.iter().for_each(|s| {
            _ = writeln!(f, "{s}");
        });

        ok()
    }
}
