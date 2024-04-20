use crate::file_lock::FileLock;
use crate::file_lock::ReadLock;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;

pub struct VenvManager {
    path: PathBuf,
    lock: FileLock,
}

impl VenvManager {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let lock_path = path.with_extension("lock");
        Ok(Self {
            path,
            lock: FileLock::new(lock_path)?,
        })
    }

    pub fn run(
        &mut self,
        python_executable: &Path,
        requirements: &str,
        args: &[String],
    ) -> anyhow::Result<ExitStatus> {
        log::debug!("Running Python in virtual environment at {:?}", self.path);
        let mut _read_lock = self.lock.read()?;

        let venv_python = self.path.join("bin").join("python");
        if !venv_python.exists() {
            _read_lock = create_venv(_read_lock, python_executable, requirements, &self.path)?;
        }

        let status = match Command::new(venv_python).args(args).status() {
            Ok(status) => status,
            Err(err) => {
                anyhow::bail!("Failed to get status from Python: {:?}", err);
            }
        };
        Ok(status)
    }

    pub fn delete(&mut self) -> anyhow::Result<()> {
        log::debug!("Deleting virtual environment at {:?}", self.path);
        let _write_lock = self.lock.write()?;
        std::fs::remove_dir_all(&self.path)?;
        Ok(())
    }
}

pub fn venv_sha(python_executable: &Path, requirements: &str) -> anyhow::Result<String> {
    let python_version = python_version(python_executable)?;
    Ok(sha256::digest(format!(
        "{python_version}\n\n{requirements}"
    )))
}

fn create_venv<'a>(
    read_lock: ReadLock<'a>,
    python_executable: &Path,
    requirements: &str,
    venv_path: &Path,
) -> anyhow::Result<ReadLock<'a>> {
    let write_lock = read_lock.upgrade()?;

    if venv_path.join("bin").join("python").exists() {
        panic!(
            "Virtual environment already exists at {:?}. This function shouldn't have been called.",
            venv_path
        );
    }

    std::fs::create_dir_all(venv_path)?;
    let status = Command::new(python_executable)
        .args(["-m", "venv"])
        .arg(venv_path)
        .status()?;
    anyhow::ensure!(status.success(), "Failed to create virtual environment");

    let requirements_file_path = venv_path.with_extension("requirements");
    {
        let mut requirements_file = File::create(&requirements_file_path)?;
        requirements_file.write_all(requirements.as_bytes())?;
    }

    let venv_pip = venv_path.join("bin").join("pip");
    let status = Command::new(venv_pip)
        .args(["install", "-r"])
        .arg(requirements_file_path)
        .status()?;
    anyhow::ensure!(
        status.success(),
        "Failed to pip install requirements into virtual environment"
    );

    write_lock.downgrade()
}

fn python_version(python_executable: &Path) -> anyhow::Result<String> {
    let output = Command::new(python_executable).arg("--version").output()?;
    Ok(String::from_utf8(output.stdout)?)
}
