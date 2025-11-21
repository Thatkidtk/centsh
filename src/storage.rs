use crate::models::Ledger;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let dirs = ProjectDirs::from("com", "centsh", "centsh")
            .context("unable to locate a config directory")?;
        let data_dir = dirs.data_dir();
        fs::create_dir_all(data_dir).context("failed to create data directory")?;
        Ok(Self {
            path: data_dir.join("ledger.json"),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Ledger> {
        if !self.path.exists() {
            return Ok(Ledger::default());
        }

        let content =
            fs::read_to_string(&self.path).with_context(|| format!("reading {:?}", self.path))?;
        let data =
            serde_json::from_str::<Ledger>(&content).context("parsing ledger data failed")?;
        Ok(data)
    }

    pub fn save(&self, ledger: &Ledger) -> Result<()> {
        let json = serde_json::to_string_pretty(ledger).context("serializing data failed")?;
        fs::write(&self.path, json).with_context(|| format!("writing {:?}", self.path))
    }
}
