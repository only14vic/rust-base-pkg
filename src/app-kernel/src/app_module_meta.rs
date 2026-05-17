use {
    crate::prelude::*,
    alloc::{
        boxed::Box,
        string::{String, ToString}
    },
    core::fmt::Debug
};

#[derive(Debug)]
pub struct AppModuleMeta<'a> {
    pub module: AppModule,
    pub name: &'a str,
    pub depends: Box<[AppModule]>,
    pub sends: Box<[&'a str]>,
    pub notifies: Box<[&'a str]>,
    pub hooks: Box<[&'a dyn AppHook]>,
    pub commands: Box<[&'a str]>
}

impl Clone for AppModuleMeta<'_> {
    fn clone(&self) -> Self {
        Self {
            module: self.module,
            name: String::leak(self.name.to_string()),
            depends: self.depends.clone(),
            sends: self
                .sends
                .iter()
                .map(|s| &*String::leak(s.to_string()))
                .collect(),
            notifies: self
                .notifies
                .iter()
                .map(|s| &*String::leak(s.to_string()))
                .collect(),
            hooks: self
                .hooks
                .iter()
                .map(|v| {
                    Box::leak(Box::new((
                        v.event(),
                        &*String::leak(v.notify().to_string())
                    ))) as &dyn AppHook
                })
                .collect(),
            commands: self
                .commands
                .iter()
                .map(|s| &*String::leak(s.to_string()))
                .collect()
        }
    }
}

impl AppModuleMeta<'_> {
    pub fn name_short(&self) -> String {
        self.name.replace("MOD_", "").to_ascii_lowercase()
    }
}
