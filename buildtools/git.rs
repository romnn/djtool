use anyhow::Result;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;

pub struct GitRepository<'a> {
    pub url: &'a str,
    pub path: &'a PathBuf,
    pub branch: Option<String>,
}

impl GitRepository<'_> {
    pub fn clone(&self) -> Result<()> {
        let _ = std::fs::remove_dir_all(&self.path);
        let mut cmd = Command::new("git");
        cmd.arg("clone").arg("--depth=1");
        if let Some(branch) = &self.branch {
            cmd.arg("-b").arg(branch);
        }

        cmd.arg(self.url).arg(self.path.to_str().unwrap());
        // println!(
        //     "cargo:warning=Cloning {} into {}",
        //     self.url,
        //     self.path.display()
        // );

        if cmd.status()?.success() {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "fetch failed").into())
        }
    }
}
