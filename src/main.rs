use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use structopt::StructOpt;

use crate::journal::Journal;

mod journal;
mod venv;

#[derive(StructOpt)]
struct Opt {
    /// The Python executable to use when creating a virtual environment.
    /// Virtual environments are considered different if created by different Pythons.
    #[structopt(long, env = "VENVCACHE_PYTHON")]
    python: PathBuf,

    /// The root of the directory tree at which we will store virtual environments.
    #[structopt(long, env = "VENVCACHE_ROOT")]
    root: PathBuf,

    /// The location of the journal file we use to communicate venv use between processes.
    #[structopt(long, env = "VENVCACHE_JOURNAL")]
    journal: PathBuf,

    #[structopt(long, default_value = "50")]
    maximum_venvs: usize,

    /// When provided, read requirements in from the provided file instead of from stdin.
    #[structopt(long)]
    requirements: Option<PathBuf>,

    /// The arguments that will be passed to the Python executable inside of the virtual environment.
    #[structopt()]
    args: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();
    let mut journal = Journal::new(&opt.journal, opt.maximum_venvs)?;

    let requirements = read_requirements(&opt)?;
    let shasum = sha256::digest(format!("{:?}:{}", opt.python, requirements));

    fs::create_dir_all(&opt.root)?;

    if let Some(venv_to_delete_shasum) = journal.record_usage(&shasum)? {
        let venv_to_delete = opt.root.join(&venv_to_delete_shasum);
        let mut venv_to_delete_lock = fd_lock::RwLock::new(File::open(venv_to_delete.with_extension(".lock"))?);
        let _write_lock = venv_to_delete_lock.write()?;
        std::fs::remove_dir_all(venv_to_delete)?;
        journal.mark_deleted(&venv_to_delete_shasum)?;
    }

    let venv_dir = opt.root.join(&shasum);
    let mut venv_rwlock = get_venv_lock(&venv_dir)?;
    for _ in 0..5 {
        {
            let _read_lock = venv_rwlock.read()?;
            if venv_dir.exists() {
                run_python(&opt, &venv_dir);
            }
        }

        {
            let _write_lock = venv_rwlock.write()?;
            create_venv(&opt, &venv_dir, &requirements)?;
        }
    }

    eprintln!("Failed to establish venv within 5 tries.");
    std::process::exit(1);
}

fn read_requirements(opt: &Opt) -> anyhow::Result<String> {
    let contents = match &opt.requirements {
        Some(path) => std::fs::read_to_string(path)?,
        None => {
            let mut contents = String::new();
            std::io::stdin().read_to_string(&mut contents)?;
            contents
        }
    };
    Ok(contents)
}

fn get_venv_lock(venv_dir: &Path) -> anyhow::Result<fd_lock::RwLock<File>> {
    let lock_name = venv_dir.with_extension("lock");
    let file = File::create(lock_name)?;
    Ok(fd_lock::RwLock::new(file))
}

fn run_python(opt: &Opt, venv_dir: &Path) -> ! {
    let venv_python = venv_dir.join("bin").join("python");
    let status = match Command::new(venv_python).args(&opt.args).status() {
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

fn create_venv(opt: &Opt, venv_dir: &Path, requirements: &str) -> anyhow::Result<()> {
    let status = Command::new(&opt.python)
        .args(["-m", "venv"])
        .arg(venv_dir)
        .status()?;
    anyhow::ensure!(status.success(), "Failed to create virtual environment");

    let requirements_file_path = venv_dir.with_extension(".requirements");
    {
        let mut requirements_file = File::create(&requirements_file_path)?;
        requirements_file.write_all(requirements.as_bytes())?;
    }

    let venv_pip = venv_dir.join("bin").join("pip");
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
