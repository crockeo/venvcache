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

    let mut journal = Journal::new(&opt.journal, opt.maximum_venvs)?;
    if let Some(venv_to_delete_sha) = journal.record_usage(&venv_sha)? {
        let delete_venv_dir = opt.root.join(&venv_to_delete_sha);
        let mut delete_manager = venv::VenvManager::new(delete_venv_dir)?;
        delete_manager.delete()?;
        journal.mark_deleted(&venv_to_delete_sha)?;
    }

    // TODO: think about how we can make this airtight.
    // right now it works like:
    // - we check if the venv exists using a read lock
    // - if it does: skip creating it
    // - and then attempt to run it
    //
    // but we drop read locks between checking, creating, and running
    // so another process could intercede and:
    //
    // - create the venv after we think it doesn't exists
    //   - this is ok, we can just make VenvManager::create idempotent
    // - or delete the venv after we think it exists
    //   - this is not ok, and i don't quite know how to fix it yet!
    //     i wish we could like """promote""" our read lock into a write lock
    //     without dropping it :(
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
