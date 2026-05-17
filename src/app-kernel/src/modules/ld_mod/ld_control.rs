use {
    crate::{modules::*, prelude::*},
    alloc::{format, sync::Weak},
    app_base::prelude::*
};

pub struct LdControl {
    app: Weak<App>
}

impl LdControl {
    pub fn load_module(&self, module_name: &'static str) -> Ok<AppModule> {
        self.app
            .upgrade()
            .ok_or("App weak is not available")?
            .notify(NT_LD_LOAD_MODULE, Some(self), Some(&module_name), None)?
            .map(|r| r.downcast::<AppModule>().map(|r| *r))
            .ok_or_else(|| format!("Invalid result of notify {}", NT_LD_LOAD_MODULE))?
    }

    pub fn load_library(&self, library_path: &'static str) -> Ok<()> {
        self.app
            .upgrade()
            .ok_or("App weak is not available")?
            .notify(NT_LD_LOAD_LIBRARY, Some(self), Some(&library_path), None)
            .void()
    }

    pub fn close_library(&self, lib_name: &'static str) -> Void {
        self.app
            .upgrade()
            .ok_or("App weak is not available")?
            .notify(NT_LD_CLOSE_LIBRARY, Some(self), Some(&lib_name), None)?;
        ok()
    }
}

impl TryFrom<&Di> for LdControl {
    type Error = Err;

    fn try_from(di: &Di) -> Result<Self, Self::Error> {
        let app = di.get_weak::<App>()?;
        Ok(Self { app })
    }
}
