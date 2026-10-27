use crate::{CancellationToken, Error, Result, check_cancel};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Programs {
    pub git: PathBuf,
    pub gh: PathBuf,
    pub cargo: PathBuf,
    pub timeout: Duration,
}
impl Default for Programs {
    fn default() -> Self {
        Self {
            git: "git".into(),
            gh: "gh".into(),
            cargo: "cargo".into(),
            timeout: Duration::from_secs(90),
        }
    }
}
impl Programs {
    pub async fn run(
        &self,
        program: &Path,
        args: &[&str],
        cwd: Option<&Path>,
        token: &CancellationToken,
    ) -> Result<Vec<u8>> {
        check_cancel(token)?;
        let mut command = tokio::process::Command::new(program);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command.spawn()?;
        let _group = Group(child.id());
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| crate::invalid("child stdout unavailable"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| crate::invalid("child stderr unavailable"))?;
        let result = tokio::select! {
            biased;
            _ = token.cancelled() => Err(Error::Cancelled),
            result = tokio::time::timeout(self.timeout, async {
                let (status,stdout,stderr) = tokio::try_join!(child.wait(),read(stdout),read(stderr))?;
                Ok::<_,std::io::Error>(std::process::Output {status,stdout,stderr})
            }) => result.map_err(|_| Error::Process { program: program.display().to_string(), message: "timed out".into() }).and_then(|result| result.map_err(Error::Io)),
        };
        let output = match result {
            Ok(output) => output,
            Err(error) => {
                _group.kill();
                let _ = child.start_kill(); // wait below reaps even an already-exited child.
                child.wait().await?;
                return Err(error);
            }
        };
        if !output.status.success() {
            return Err(Error::Process {
                program: program.display().to_string(),
                message: format!(
                    "{}: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            });
        }
        Ok(output.stdout)
    }
    pub(crate) async fn probe(
        &self,
        program: &Path,
        args: &[&str],
        token: &CancellationToken,
    ) -> Result<Option<String>> {
        match self.run(program, args, None, token).await {
            Ok(bytes) => Ok(String::from_utf8(bytes)
                .ok()
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())),
            Err(Error::Cancelled) => Err(Error::Cancelled),
            Err(_) => Ok(None),
        }
    }
}
struct Group(Option<u32>);
impl Group {
    fn kill(&self) {
        #[cfg(unix)]
        if let Some(pid) = self.0.and_then(|pid| i32::try_from(pid).ok()) {
            // SAFETY: the child was launched in its own group. The numeric
            // negative PGID addresses that group without dereferencing memory.
            // Descendants retaining captured pipes are terminated on shutdown.
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
        }
    }
}
impl Drop for Group {
    fn drop(&mut self) {
        self.kill();
    }
}
async fn read(mut reader: impl tokio::io::AsyncRead + Unpin) -> std::io::Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await?;
    Ok(bytes)
}
