use {crate::prelude::AppModuleMeta, alloc::boxed::Box};

#[derive(Debug, Clone)]
pub struct LdMeta<'a> {
    pub lib: Box<str>,
    pub mods: Box<[AppModuleMeta<'a>]>
}
