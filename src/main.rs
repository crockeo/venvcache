use std::io::Read;
use std::path::PathBuf;
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
    pretty_env_logger::init();

    let opt = Opt::from_args();
    std::fs::create_dir_all(&opt.root)?;

    let requirements = read_requirements(&opt)?;

    let venv_sha = venv::venv_sha(&opt.python, &requirements)?;
    let venv_dir = opt.root.join(&venv_sha);
    let mut manager = venv::VenvManager::new(venv_dir)?;

    // I can't find a good way to do locking here such that
    // we never drop a lock between creating the venv + attempting a run
    // because there is no mechanism to atomically downgrade
    // an exclusive lock to a shared lock.
    //
    // Instead: we just attempt to make the venv several times,
    // and hope that at least one time the venv we care about isn't lost.
    for _ in 0..5 {
        match manager.run(&opt.args) {
            Ok(status) => std::process::exit(status.code().unwrap_or(1)),
            Err(e) if e.downcast_ref::<venv::Error>() == Some(&venv::Error::MissingVenv) => {
                log::debug!("Virtual environment doesn't exist. Creating a new one.");
            }
            Err(e) => return Err(e),
        }
        manager.create(&opt.python, &requirements)?;
    }

    let journal = Journal::new(&opt.journal, opt.maximum_venvs)?;
    let expired_venvs = journal.record_usage(&venv_sha)?;
    for expired_venv in expired_venvs {
        let expired_venv_dir = opt.root.join(&expired_venv);
        let mut expired_manager = venv::VenvManager::new(expired_venv_dir)?;
        expired_manager.delete()?;
        journal.mark_deleted(&expired_venv)?;
    }

    log::error!("Failed to create a venv within 5 attempts.");
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
