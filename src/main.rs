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

    // Can we leak a venv on disk without tracking it?
    //
    // - p1 marks that `venv_sha` is in use and finds a sha to delete
    // - p2 marks some othe sha, and gets `venv_sha` as its venv_to_delete
    // - p2 deletes the venv under `venv_sha` and marks that it's deleted. it gets removed from the database
    // - p1 attempts to run, does not have a venv
    // - p1 creates a venv
    //
    // So:
    // - the venv for p1 exists on disk
    // - the record of it doesn't exist in the database
    // TODO: fix this bug! ^
    let mut journal = Journal::new(&opt.journal, opt.maximum_venvs)?;
    if let Some(venv_to_delete_sha) = journal.record_usage(&venv_sha)? {
        let delete_venv_dir = opt.root.join(&venv_to_delete_sha);
        let mut delete_manager = venv::VenvManager::new(delete_venv_dir)?;
        delete_manager.delete()?;
        journal.mark_deleted(&venv_to_delete_sha)?;
    }

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
