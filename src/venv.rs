use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;
use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum Error {
    #[error("The requested venv didn't exist when we attempted to execute something using it.")]
    MissingVenv,
}

pub struct VenvManager {
    path: PathBuf,
    lock: fd_lock::RwLock<File>,
}

impl VenvManager {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let lock_path = path.with_extension("lock");
        Ok(Self {
            path,
            lock: fd_lock::RwLock::new(File::create(lock_path)?),
        })
    }

    pub fn create(&mut self, python_executable: &Path, requirements: &str) -> anyhow::Result<()> {
        log::debug!("Creating virtual environment at {:?}", self.path);
        let _write_lock = self.lock.write()?;
        if self.path.join("bin").join("python").exists() {
            log::debug!("Virtual environment already exists at {:?}", self.path);
            return Ok(());
        }

        std::fs::create_dir_all(&self.path)?;
        let status = Command::new(python_executable)
            .args(["-m", "venv"])
            .arg(&self.path)
            .status()?;
        anyhow::ensure!(status.success(), "Failed to create virtual environment");

        let requirements_file_path = self.path.with_extension("requirements");
        {
            let mut requirements_file = File::create(&requirements_file_path)?;
            requirements_file.write_all(requirements.as_bytes())?;
        }

        let venv_pip = self.path.join("bin").join("pip");
        let status = Command::new(&venv_pip)
            .args(&["install", "-r"])
            .arg(requirements_file_path)
            .status()?;
        anyhow::ensure!(
            status.success(),
            "Failed to pip install requirements into virtual environment"
        );

        Ok(())
    }

    pub fn delete(&mut self) -> anyhow::Result<()> {
        log::debug!("Deleting virtual environment at {:?}", self.path);
        todo!()
    }

    pub fn run(&self, args: &[String]) -> anyhow::Result<ExitStatus> {
        log::debug!("Running Python in virtual environment at {:?}", self.path);

        let venv_python = self.path.join("bin").join("python");
        if !venv_python.exists() {
            return Err(Error::MissingVenv.into());
        }

        let status = match Command::new(venv_python).args(args).status() {
            Ok(status) => status,
            Err(err) => {
                eprintln!("Failed to get status from Python: {:?}", err);
                std::process::exit(1)
            }
        };
        match status.code() {
            Some(code) => std::process::exit(code),
            None => {
                eprintln!("Python subprocess terminated by a signal.");
                std::process::exit(127)
            }
        }
    }
}

pub fn venv_sha(python_executable: &Path, requirements: &str) -> anyhow::Result<String> {
    let python_version = python_version(python_executable)?;
    Ok(sha256::digest(format!(
        "{python_version}\n\n{requirements}"
    )))
}

fn python_version(python_executable: &Path) -> anyhow::Result<String> {
    let output = Command::new(python_executable).arg("--version").output()?;
    Ok(String::from_utf8(output.stdout)?)
}
