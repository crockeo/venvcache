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

    /// When provided, use the contents of this argument as the venv's requirements.
    #[structopt(long, env = "VENVCACHE_REQUIREMENTS")]
    requirements: Option<String>,

    /// When provided, read requirements in from the provided file instead of from stdin.
    #[structopt(long)]
    requirements_path: Option<PathBuf>,

    /// The arguments that will be passed to the Python executable inside of the virtual environment.
    #[structopt()]
    args: Vec<String>,
}

impl Opt {
    fn requirements_source(&self) -> anyhow::Result<RequirementsSource> {
        let source = match (&self.requirements, &self.requirements_path) {
            (Some(_), Some(_)) => anyhow::bail!(""),
            (Some(requirements), None) => RequirementsSource::Provided(requirements.to_owned()),
            (None, Some(requirements_path)) => {
                RequirementsSource::File(requirements_path.to_owned())
            }
            (None, None) => RequirementsSource::Stdin,
        };
        Ok(source)
    }
}

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let opt = Opt::from_args();
    std::fs::create_dir_all(&opt.root)?;

    let requirements = opt.requirements_source()?.read_requirements()?;

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

enum RequirementsSource {
    Stdin,
    Provided(String),
    File(PathBuf),
}

impl RequirementsSource {
    fn read_requirements(self) -> anyhow::Result<String> {
        use RequirementsSource::*;
        let contents = match self {
            Stdin => {
                let mut contents = String::new();
                std::io::stdin().read_to_string(&mut contents)?;
                contents
            }
            Provided(contents) => contents,
            File(path) => std::fs::read_to_string(path)?,
        };
        Ok(contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn test_read_requirements_provided() -> anyhow::Result<()> {
        let source = RequirementsSource::Provided("requests==1.2.3\n".to_owned());
        let requirements = source.read_requirements()?;
        assert_eq!(requirements, "requests==1.2.3\n");
        Ok(())
    }

    #[test]
    fn test_read_requirements_file() -> anyhow::Result<()> {
        let tempdir = TempDir::new("venvcache-test")?;
        let requirements_path = tempdir.path().join("requirements.txt");
        std::fs::write(&requirements_path, b"requests==4.5.6\n")?;

        let source = RequirementsSource::File(requirements_path);
        let requirements = source.read_requirements()?;
        assert_eq!(requirements, "requests==4.5.6\n");
        Ok(())
    }
}
