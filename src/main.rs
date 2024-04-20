use std::io::Read;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::journal::Journal;

mod file_lock;
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

    let status = manager.run(&opt.python, &requirements, &opt.args)?;
    let Some(status_code) = status.code() else {
        log::error!("Failed to create venv + run Python: {:?}", status);
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
