use {
    alloc::{format, rc::Rc},
    app_base::prelude::*,
    core::{
        any::Any,
        cell::{Cell, RefCell},
        fmt::{Debug, Display}
    }
};

#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AppEvent {
    LOAD,
    UNLOAD,
    #[default]
    NOTIFY,
    HOOK,
    META
}

impl Display for AppEvent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct AppEventData<'e> {
    pub event: AppEvent,
    pub notify: &'e str,
    pub sender: Option<&'e dyn TypedAny>,
    pub context: Option<&'e dyn TypedAny>,
    pub result: RefCell<Option<Rc<dyn TypedAny>>>,
    pub handled: Cell<bool>
}

impl AppEventData<'_> {
    pub fn sender_as<T: Any>(&self) -> Ok<&T> {
        self.sender
            .map(|v| v.downcast::<T>())
            .transpose()?
            .ok_or_else(|| format!("{}:{} Sender is empty", self.event, self.notify))?
            .into_ok()
    }

    pub fn context_as<T: Any>(&self) -> Ok<&T> {
        self.context
            .map(|v| v.downcast::<T>())
            .transpose()?
            .ok_or_else(|| format!("{}:{} Context is empty", self.event, self.notify))?
            .into_ok()
    }

    pub fn result_as<T: Any>(&self) -> Ok<Option<Rc<T>>> {
        self.result
            .try_borrow()?
            .clone()
            .map(|v| v.downcast::<T>())
            .transpose()?
            .ok_or_else(|| format!("{}:{} Result is empty", self.event, self.notify))?
            .into_ok()
    }

    pub fn set_result<T: TypedAny>(&self, value: T) -> Void {
        *self.result.try_borrow_mut()? = Some(Rc::from(value));
        ok()
    }
}

impl<'e> From<&'e str> for AppEventData<'e> {
    fn from(value: &'e str) -> Self {
        Self { event: AppEvent::NOTIFY, notify: value, ..Default::default() }
    }
}

impl From<AppEvent> for AppEventData<'_> {
    fn from(value: AppEvent) -> Self {
        Self { event: value, ..Default::default() }
    }
}

#[derive(Debug)]
pub struct AppHookContext<'e> {
    pub origin_event: &'e AppEventData<'e>,
    pub is_pre_hook: Cell<bool>
}
