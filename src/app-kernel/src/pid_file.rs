use {
    crate::prelude::AppConfig,
    alloc::{boxed::Box, ffi::CString, format},
    app_base::prelude::*
};

#[derive(Debug)]
pub struct PidFile {
    pub file_path: Box<str>,
    fd: Mutex<Option<libc::c_int>>
}

impl PidFile {
    pub fn is_created(&self) -> bool {
        self.fd.lock().is_some()
    }

    pub fn create(&self) -> Ok<bool> {
        if self.is_created() {
            return Ok(false);
        }

        if self.file_path.is_empty() {
            return Ok(false);
        }

        let pid_file_str = &*self.file_path;
        let pid_file = CString::new(pid_file_str)?;

        Dirs::mkdir(Dirs::dirname(pid_file_str))?;

        unsafe {
            let fd = libc::open(
                pid_file.as_ptr(),
                libc::O_CREAT | libc::O_WRONLY | libc::O_NONBLOCK,
                libc::S_IRUSR | libc::S_IWUSR | libc::S_IRGRP | libc::S_IROTH
            );

            if fd == -1 {
                return Err(format!("Could not create pid file: {pid_file_str}"))?;
            }

            if libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) != 0 {
                libc::close(fd);
                return Err(format!("Could not lock pid file: {pid_file_str}"))?;
            }

            let pid_str = &mut [0i8; 30] as *mut _;
            libc::snprintf(pid_str, 30, c"%d\n".as_ptr(), libc::getpid());
            let pid_len = libc::strlen(pid_str);

            if libc::ftruncate(fd, 0) != 0 {
                libc::flock(fd, libc::LOCK_UN);
                libc::close(fd);
                return Err(format!("Could not truncate pid file: {pid_file_str}"))?;
            }

            if libc::write(fd, pid_str.cast(), pid_len) != pid_len as isize {
                libc::flock(fd, libc::LOCK_UN);
                libc::close(fd);
                return Err(format!("Could not write to pid file: {pid_file_str}"))?;
            }

            libc::fsync(fd);

            *self.fd.lock() = Some(fd);

            Env::is_debug().then(|| log::debug!("PID file created: {pid_file_str}"));
        }

        Ok(true)
    }

    pub fn delete(&self) -> Void {
        let Some(fd) = self.fd.lock().take() else { return ok() };

        let pid_file_str = &*self.file_path;
        let pid_file = CString::new(pid_file_str)?;

        unsafe {
            libc::flock(fd, libc::LOCK_UN);
            libc::close(fd);
            libc::remove(pid_file.as_ptr());
        }

        Env::is_debug().then(|| log::debug!("PID file removed: {pid_file_str}"));

        ok()
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        self.delete().ok();
    }
}

impl TryFrom<&Di> for PidFile {
    type Error = Err;

    fn try_from(di: &Di) -> Result<Self, Self::Error> {
        let app_config = di.get::<AppConfig>()?;
        let pid_file = Self {
            file_path: app_config.pid_file.clone(),
            fd: Default::default()
        };
        Ok(pid_file)
    }
}
