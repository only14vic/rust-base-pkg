#[macro_export]
macro_rules! app_cdylib_mod {
    ($link_name:literal, $visible:vis $module_name:ident, $module_path:path) => {
        #[allow(non_snake_case)]
        $visible const fn $module_name() -> ::app_kernel::prelude::AppModule {
            #[cfg(all(not(target_env = "musl"), prefer_dynamic))]
            {
                #[link(name = $link_name)]
                unsafe extern "C" {
                    static $module_name: ::app_kernel::prelude::AppModuleC;
                }

                return unsafe { ::core::mem::transmute($module_name) };
            }

            #[cfg(any(target_env = "musl", not(prefer_dynamic)))]
            return $module_path;
        }
    };
}

#[macro_export]
macro_rules! app_module_meta {
    ($meta:expr) => {
        fn meta() -> &'static AppModuleMeta<'static> {
            static META: Once<AppModuleMeta<'static>> = Once::new();
            META.call_once(|| $meta)
        }
    };
}

#[macro_export]
macro_rules! app_module_name {
    ($const:ident,no_mangle, $(#[cfg($cfg:meta)])?) => {{
        $(#[cfg($cfg)])?
        mod external {
            #[unsafe(no_mangle)]
            static $const: super::AppModule = super::$const;
        }
        app_module_name!($const)
    }};
    ($const:ident,no_mangle) => {{
        mod external {
            #[unsafe(no_mangle)]
            static $const: super::AppModule = super::$const;
        }
        app_module_name!($const)
    }};
    ($const:ident) => {{
        let _ = $const;
        stringify!($const)
    }};
}

#[macro_export]
macro_rules! ld_meta {
    ($meta:expr) => {
        #[allow(improper_ctypes_definitions)]
        #[unsafe(no_mangle)]
        extern "C" fn __ld_meta() -> *const LdMeta<'static> {
            static META: Once<LdMeta<'static>> = Once::new();
            META.call_once(|| $meta)
        }
    };
}
