use {crate::prelude::AppConfig, app_base::prelude::*, core::ops::Not};

pub trait CmdArgsExt {
    fn is_force(&self) -> bool;
    fn is_debug(&self) -> bool;
    fn is_verbose(&self) -> bool;
    fn is_quiet(&self) -> bool;
    fn do_help(&self) -> bool;
    fn do_version(&self) -> bool;
}

impl CmdArgsExt for CmdArgs {
    fn is_force(&self) -> bool {
        self.get_flag(AppConfig::OPT_FORCE)
    }

    fn is_debug(&self) -> bool {
        self.get_flag(AppConfig::OPT_DEBUG)
    }

    fn is_verbose(&self) -> bool {
        self.get_flag(AppConfig::OPT_VERBOSE)
    }

    fn is_quiet(&self) -> bool {
        self.get_flag(AppConfig::OPT_VERBOSE).not()
    }

    fn do_help(&self) -> bool {
        self.get_flag(AppConfig::OPT_HELP)
    }

    fn do_version(&self) -> bool {
        self.get_flag(AppConfig::OPT_VERSION)
    }
}
