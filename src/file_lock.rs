//! Provides a file locking interface for files on POSIX-compliant operating systems.
//! Very similar to the open source fd-lock library,
//! except that it supports atomically upgrading / downgrading the lock.
use libc::fcntl;
use libc::flock;
use libc::F_SETLKW;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::Path;

pub struct FileLock {
    file: File,
}

impl FileLock {
    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = File::create(path)?;
        Ok(Self { file })
    }

    pub fn read_lock(&mut self) -> anyhow::Result<ReadLock> {
        ReadLock::new(&mut self.file)
    }

    pub fn write_lock(&mut self) -> anyhow::Result<WriteLock> {
        WriteLock::new(&mut self.file)
    }
}

pub struct ReadLock<'a> {
    file: Option<&'a mut File>,
}

impl<'a> ReadLock<'a> {
    fn new(file: &'a mut File) -> anyhow::Result<Self> {
        apply_lock(file, LockOperation::Read)?;
        Ok(Self { file: Some(file) })
    }

    pub fn upgrade(mut self) -> anyhow::Result<WriteLock<'a>> {
        let file = self.file.take().unwrap();
        WriteLock::new(file)
    }
}

impl Drop for ReadLock<'_> {
    fn drop(&mut self) {
        if let Some(ref mut file) = self.file {
            apply_lock(file, LockOperation::Unlock).expect("Failed to unlock file during ReadLock Drop");
        }
    }
}

pub struct WriteLock<'a> {
    file: Option<&'a mut File>,
}

impl<'a> WriteLock<'a> {
    fn new(file: &'a mut File) -> anyhow::Result<Self> {
        apply_lock(file, LockOperation::Write)?;
        Ok(Self { file: Some(file) })
    }

    pub fn downgrade(mut self) -> anyhow::Result<ReadLock<'a>> {
        let file = self.file.take().unwrap();
        ReadLock::new(file)
    }
}

impl Drop for WriteLock<'_> {
    fn drop(&mut self) {
        if let Some(ref mut file) = self.file {
            apply_lock(file, LockOperation::Unlock).expect("Failed to unlock file during WriteLock Drop");
        }
    }
}

enum LockOperation {
    Read,
    Write,
    Unlock,
}

fn apply_lock(file: &mut File, operation: LockOperation) -> anyhow::Result<()> {
    let fd = file.as_raw_fd();
    let lock_type = match operation {
        LockOperation::Read => libc::F_RDLCK,
        LockOperation::Write => libc::F_WRLCK,
        LockOperation::Unlock => libc::F_UNLCK,
    };
    let result = unsafe {
        fcntl(
            fd,
            F_SETLKW,
            &flock {
                l_type: lock_type as i16,
                l_whence: libc::SEEK_SET as i16,
                l_start: 0,
                l_len: 0,
                l_pid: 0,
            },
        )
    };
    if result == -1 {
        anyhow::bail!("Failed to lock file");
    }
    Ok(())
}
