/// Provides a file locking interface for files on POSIX-compliant operating systems.
/// Very similar to the open source fd-lock library,
/// except that it supports atomically upgrading / downgrading the lock.
use std::fs::File;

pub struct FileLock {
    file: File,
}

impl FileLock {
    pub fn new() -> anyhow::Result<Self> {
        todo!()
    }

    pub fn read_lock(&mut self) -> anyhow::Result<ReadLock> {
        todo!()
    }

    pub fn write_lock(&mut self) -> anyhow::Result<WriteLock> {
        todo!()
    }
}

pub struct ReadLock<'a> {
    file: &'a mut File,
}

impl<'a> ReadLock<'a> {
    pub fn upgrade(self) -> WriteLock<'a> {
        todo!()
    }
}

impl Drop for ReadLock<'_> {
    fn drop(&mut self) {}
}

pub struct WriteLock<'a> {
    file: &'a mut File,
}

impl<'a> WriteLock<'a> {
    pub fn downgrade(self) -> ReadLock<'a> {
        todo!()
    }
}

impl Drop for WriteLock<'_> {
    fn drop(&mut self) {}
}
