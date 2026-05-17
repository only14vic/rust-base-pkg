use {
    app_base::prelude::*,
    futures::{FutureExt, future::LocalBoxFuture},
    std::sync::Arc
};

type InitItem = Arc<dyn Fn(&Di) -> LocalBoxFuture<'static, Void> + Send + Sync>;

#[derive(Default)]
pub struct AsyncInit(RwLock<Vec<InitItem>>);

impl AsyncInit {
    pub fn add(&self, item: impl AsyncFn(&Di) -> Void + Send + Sync + 'static) -> &Self {
        let item = Arc::new(item);
        self.0.write().push(Arc::new(move |di| {
            let item = item.clone();
            let di = unsafe { di.as_static() };
            async move { item(di).await }.boxed_local()
        }));
        self
    }

    pub async fn init(self, di: &Di) -> Void {
        Env::is_debug().then(|| log::trace!("async InitAsyncRuntime::init()"));

        let list = core::mem::take(&mut *self.0.write());
        for item in list {
            item(di).await.unwrap();
        }

        ok()
    }
}

impl TryFrom<&Di> for AsyncInit {
    type Error = Err;

    fn try_from(_di: &Di) -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
}
