use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;

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
        let _write_lock = self.lock.write()?;

        std::fs::create_dir_all(&self.path)?;
        let status = Command::new(python_executable)
            .args(["-m", "venv"])
            .arg(&self.path)
            .status()?;
        anyhow::ensure!(status.success(), "Failed to create virtual environment");

        let requirements_file_path = self.path.with_extension(".requirements");
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
        todo!()
    }

    pub fn run(&self, args: &[String]) -> anyhow::Result<ExitStatus> {
        let venv_python = self.path.join("bin").join("python");
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
