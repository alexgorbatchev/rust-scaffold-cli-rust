use crate::{CancellationToken, Error, Plan, Result, check_cancel, invalid};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, File, OpenOptions},
};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    io::{self, Write},
    path::{Component, Path, PathBuf},
};

struct WorkDirectory {
    parent: Dir,
    directory: Dir,
    display: PathBuf,
    recovery: bool,
}

struct PreparedFile {
    work: usize,
    name: OsString,
    staged: OsString,
    backup: Option<OsString>,
    committed: bool,
}

/// Each directory is opened without following symlinks and retained until the
/// transaction ends. No commit or rollback re-resolves an ambient project path.
/// Work directories deliberately have no destructive Drop: an abandoned or
/// failed rollback must leave its original files available for recovery.
pub(super) struct Transaction {
    work: Vec<WorkDirectory>,
    files: Vec<PreparedFile>,
    created: Vec<(Dir, OsString)>,
}

impl Transaction {
    pub(super) fn prepare(plan: &Plan, token: &CancellationToken) -> Result<Self> {
        validate_paths(plan)?;
        check_cancel(token)?;
        let mut transaction = Self {
            work: Vec::new(),
            files: Vec::new(),
            created: Vec::new(),
        };
        match transaction.prepare_files(plan, token) {
            Ok(()) => Ok(transaction),
            Err(error) => Err(transaction.rollback(error)),
        }
    }

    fn prepare_files(&mut self, plan: &Plan, token: &CancellationToken) -> Result<()> {
        let target = self.open_target(&plan.options.target)?;
        if !plan.options.force && target.entries()?.next().transpose()?.is_some() {
            return Err(invalid(
                "target directory is not empty (use --force to overwrite)",
            ));
        }
        let mut parents = BTreeMap::new();
        for (path, bytes) in &plan.files {
            check_cancel(token)?;
            let parent_path = path
                .parent()
                .ok_or_else(|| invalid("generated file has no parent"))?;
            let work = match parents.get(parent_path) {
                Some(index) => *index,
                None => {
                    let parent = self.open_relative(&target, parent_path)?;
                    let index =
                        self.new_work_directory(parent, &plan.options.target.join(parent_path))?;
                    parents.insert(parent_path.to_owned(), index);
                    index
                }
            };
            let name = path
                .file_name()
                .ok_or_else(|| invalid("generated file has no name"))?;
            let original = regular_file(&self.work[work].parent, name)?;
            let staged: OsString = format!("staged-{}", self.files.len()).into();
            let mut options = OpenOptions::new();
            options
                .write(true)
                .create_new(true)
                .follow(FollowSymlinks::No);
            #[cfg(unix)]
            {
                use cap_std::fs::OpenOptionsExt;
                options.mode(0o644);
            }
            let mut file = self.work[work].directory.open_with(&staged, &options)?;
            file.write_all(bytes)?;
            if let Some(original) = original {
                preserve_metadata(&original, &file)?;
            }
            file.sync_all()?;
            self.files.push(PreparedFile {
                work,
                name: name.into(),
                staged,
                backup: None,
                committed: false,
            });
        }
        Ok(())
    }

    fn open_target(&mut self, path: &Path) -> Result<Dir> {
        let mut root = PathBuf::new();
        let mut relative = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => root.push(component.as_os_str()),
                Component::Normal(name) => relative.push(name),
                _ => return Err(invalid("target must be an absolute normalized directory")),
            }
        }
        if !path.is_absolute() {
            return Err(invalid("target must be an absolute normalized directory"));
        }
        let root = Dir::open_ambient_dir(root, ambient_authority())?;
        self.open_relative(&root, &relative)
    }

    fn open_relative(&mut self, root: &Dir, path: &Path) -> Result<Dir> {
        let mut directory = root.try_clone()?;
        for component in path.components() {
            let Component::Normal(name) = component else {
                return Err(invalid("directory path must stay within the project"));
            };
            directory = match directory.open_dir_nofollow(name) {
                Ok(child) => child,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    match directory.create_dir(name) {
                        Ok(()) => self.created.push((directory.try_clone()?, name.into())),
                        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                        Err(error) => return Err(error.into()),
                    }
                    directory.open_dir_nofollow(name)?
                }
                Err(error) => return Err(error.into()),
            };
        }
        Ok(directory)
    }

    fn new_work_directory(&mut self, parent: Dir, path: &Path) -> Result<usize> {
        let name = format!(".rust-scaffold-{}", uuid::Uuid::new_v4());
        let mut builder = cap_std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use cap_std::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        parent.create_dir_with(&name, &builder)?;
        let directory = match parent.open_dir_nofollow(&name) {
            Ok(directory) => directory,
            Err(error) => {
                // Nothing has been backed up yet, but report a cleanup failure.
                return match parent.remove_dir(&name) {
                    Ok(()) => Err(error.into()),
                    Err(cleanup) => Err(Error::Published(format!(
                        "{error}; removing {}: {cleanup}",
                        path.join(name).display()
                    ))),
                };
            }
        };
        let index = self.work.len();
        self.work.push(WorkDirectory {
            parent,
            directory,
            display: path.join(name),
            recovery: false,
        });
        Ok(index)
    }

    pub(super) fn commit_file(&mut self, index: usize) -> Result<()> {
        let file = &mut self.files[index];
        if file.committed {
            return Ok(());
        }
        let work = &self.work[file.work];
        // Opening the final entry without following it also rejects a directory
        // or symlink installed by another process since preparation.
        if regular_file(&work.parent, &file.name)?.is_some() {
            let backup: OsString = format!("original-{index}").into();
            work.parent.rename(&file.name, &work.directory, &backup)?;
            file.backup = Some(backup);
        }
        work.directory
            .rename(&file.staged, &work.parent, &file.name)?;
        file.committed = true;
        Ok(())
    }

    pub(super) fn commit_remaining(mut self, token: &CancellationToken) -> Result<()> {
        for index in 0..self.files.len() {
            if let Err(error) = check_cancel(token).and_then(|()| self.commit_file(index)) {
                return Err(self.rollback(error));
            }
        }
        self.finish()
    }

    pub(super) fn rollback(mut self, error: Error) -> Error {
        let mut failures = Vec::new();
        for file in self.files.iter().rev() {
            let work = &mut self.work[file.work];
            let restored = if let Some(backup) = &file.backup {
                work.directory.rename(backup, &work.parent, &file.name)
            } else if file.committed {
                match work.parent.remove_file(&file.name) {
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                    result => result,
                }
            } else {
                Ok(())
            };
            if let Err(restore) = restored {
                work.recovery = true;
                failures.push(format!(
                    "restoring {}: {restore}; recovery directory {}",
                    file.name.to_string_lossy(),
                    work.display.display()
                ));
            }
        }
        self.cleanup_work(&mut failures);
        for (parent, name) in self.created.into_iter().rev() {
            if let Err(cleanup) = parent.remove_dir(&name) {
                failures.push(format!(
                    "removing created directory {}: {cleanup}",
                    name.to_string_lossy()
                ));
            }
        }
        if failures.is_empty() {
            error
        } else {
            Error::Published(format!("{error}; rollback failed: {}", failures.join("; ")))
        }
    }

    pub(super) fn finish(mut self) -> Result<()> {
        let mut failures = Vec::new();
        self.cleanup_work(&mut failures);
        if failures.is_empty() {
            Ok(())
        } else {
            Err(Error::Published(format!(
                "cleanup failed: {}",
                failures.join("; ")
            )))
        }
    }

    fn cleanup_work(&mut self, failures: &mut Vec<String>) {
        for work in self.work.drain(..) {
            if !work.recovery
                && let Err(error) = work.directory.remove_open_dir_all()
            {
                failures.push(format!("removing {}: {error}", work.display.display()));
            }
        }
    }
}

fn validate_paths(plan: &Plan) -> Result<()> {
    for path in plan.files.keys() {
        if path.as_os_str().is_empty()
            || path
                .components()
                .any(|p| !matches!(p, Component::Normal(_)))
        {
            return Err(invalid(
                "generated path must be relative and confined to the project",
            ));
        }
        if path
            .ancestors()
            .skip(1)
            .any(|parent| plan.files.contains_key(parent))
        {
            return Err(invalid("a generated file is also used as a directory"));
        }
    }
    Ok(())
}

fn regular_file(directory: &Dir, name: &OsStr) -> Result<Option<File>> {
    match directory.symlink_metadata(name) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => {
            return Err(invalid(format!(
                "not a regular file: {}",
                name.to_string_lossy()
            )));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = directory.open_with(name, &options)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("destination changed to a non-regular file"));
    }
    Ok(Some(file))
}

fn preserve_metadata(original: &File, replacement: &File) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::{fd::AsRawFd, unix::fs::MetadataExt};
        use xattr::FileExt;
        let original = original.try_clone()?.into_std();
        let replacement = replacement.try_clone()?.into_std();
        let metadata = original.metadata()?;
        let current = replacement.metadata()?;
        if metadata.uid() != current.uid() || metadata.gid() != current.gid() {
            // SAFETY: the replacement File owns this live descriptor; fchown
            // receives only numeric owner/group IDs obtained from file metadata.
            if unsafe { libc::fchown(replacement.as_raw_fd(), metadata.uid(), metadata.gid()) }
                == -1
            {
                return Err(io::Error::last_os_error().into());
            }
        }
        for name in original.list_xattr()? {
            if let Some(value) = original.get_xattr(&name)? {
                replacement.set_xattr(name, &value)?;
            }
        }
        replacement.set_permissions(metadata.permissions())?;
    }
    #[cfg(not(unix))]
    replacement.set_permissions(original.metadata()?.permissions())?;
    Ok(())
}
