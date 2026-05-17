use {crate::prelude::*, core::fmt::Debug};

pub trait AppHook: Debug + Send + Sync {
    fn event(&self) -> AppEvent;

    fn notify(&self) -> &str;
}

impl AppHook for &str {
    fn event(&self) -> AppEvent {
        AppEvent::NOTIFY
    }

    fn notify(&self) -> &str {
        self
    }
}

impl AppHook for AppEvent {
    fn event(&self) -> AppEvent {
        *self
    }

    fn notify(&self) -> &str {
        ""
    }
}

impl AppHook for (AppEvent, &str) {
    fn event(&self) -> AppEvent {
        self.0
    }

    fn notify(&self) -> &str {
        self.1
    }
}

impl AppHook for (AppEvent, AppModule) {
    fn event(&self) -> AppEvent {
        self.0
    }

    fn notify(&self) -> &str {
        let meta = App::meta(self.1);
        meta.name
    }
}
