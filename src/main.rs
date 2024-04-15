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

    let requirements = read_requirements(&opt.requirements)?;

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
    let mut status_code: Option<i32> = None;
    for _ in 0..5 {
        let result = manager.run(&opt.args);
        if let Ok(status) = result {
            status_code = Some(status.code().unwrap_or(1));
            break;
        }

        let err = result.err().unwrap();
        if err.downcast_ref::<venv::Error>() == Some(&venv::Error::MissingVenv) {
            log::debug!("Virtual environment doesn't exist. Creating a new one.");
            manager.create(&opt.python, &requirements)?;
            continue;
        }

        return Err(err);
    }

    let Some(status_code) = status_code else {
        log::error!("Failed to create a venv within 5 attempts.");
        std::process::exit(127);
    };

    let journal = Journal::new(&opt.journal, opt.maximum_venvs)?;
    let expired_venvs = journal.record_usage(&venv_sha)?;
    for expired_venv in expired_venvs {
        let expired_venv_dir = opt.root.join(&expired_venv);
        let mut expired_manager = venv::VenvManager::new(expired_venv_dir)?;
        expired_manager.delete()?;
        journal.mark_deleted(&expired_venv)?;
    }

    std::process::exit(status_code)
}

fn read_requirements(requirements: &Option<PathBuf>) -> anyhow::Result<String> {
    let contents = match requirements {
        Some(path) => std::fs::read_to_string(path)?,
        None => {
            let mut contents = String::new();
            std::io::stdin().read_to_string(&mut contents)?;
            contents
        }
    };
    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn test_read_requirements_file() -> anyhow::Result<()> {
        let tempdir = TempDir::new("venvcache-test")?;
        let requirements_path = tempdir.path().join("requirements.txt");
        std::fs::write(&requirements_path, b"requests\n")?;

        let requirements = read_requirements(&Some(requirements_path))?;
        assert_eq!(requirements, "requests\n");
        Ok(())
    }
}
