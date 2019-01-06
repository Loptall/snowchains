use crate::errors::{FileErrorKind, FileResult, StdError};
use crate::path::{AbsPath, AbsPathBuf};

use failure::{Fail, Fallible, ResultExt};
use fs2::FileExt;
use serde::de::DeserializeOwned;
use serde::Serialize;

use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};

pub(crate) fn create_dir_all(dir: &AbsPath) -> FileResult<()> {
    std::fs::create_dir_all(dir)
        .map_err(|e| e.context(FileErrorKind::CreateDir(dir.to_owned())).into())
}

pub(crate) fn write(path: &AbsPath, contents: &[u8]) -> FileResult<()> {
    create_file_and_dirs(path)?
        .write_all(contents)
        .map_err(|e| e.context(FileErrorKind::Write(path.to_owned())).into())
}

pub(crate) fn write_json_pretty(
    path: &AbsPath,
    json: &(impl Serialize + ?Sized),
) -> FileResult<()> {
    let json = serde_json::to_string_pretty(json)
        .with_context(|_| FileErrorKind::Write(path.to_owned()))?;
    write(path, json.as_bytes())
}

pub(crate) fn read_to_string(path: &AbsPath) -> FileResult<String> {
    std::fs::read_to_string(path)
        .map_err(|e| e.context(FileErrorKind::Read(path.to_owned())).into())
}

pub(crate) fn read_json<T: DeserializeOwned>(path: &AbsPath) -> FileResult<T> {
    File::open(path)
        .map_err(failure::Error::from)
        .and_then(|f| serde_json::from_reader(f).map_err(|e| StdError::from(e).into()))
        .map_err(|e| e.context(FileErrorKind::Read(path.to_owned())).into())
}

pub(crate) fn read_toml<T: DeserializeOwned>(path: &AbsPath) -> FileResult<T> {
    std::fs::read_to_string(path)
        .map_err(failure::Error::from)
        .and_then(|s| toml::from_str(&s).map_err(|e| StdError::from(e).into()))
        .map_err(|e| e.context(FileErrorKind::Read(path.to_owned())).into())
}

pub(crate) fn read_yaml<T: DeserializeOwned>(path: &AbsPath) -> FileResult<T> {
    std::fs::read_to_string(path)
        .map_err(failure::Error::from)
        .and_then(|s| serde_yaml::from_str(&s).map_err(|e| StdError::from(e).into()))
        .map_err(|e| e.context(FileErrorKind::Read(path.to_owned())).into())
}

pub(crate) fn create_file_and_dirs(path: &AbsPath) -> FileResult<File> {
    if let Some(dir) = path.parent() {
        if !dir.exists() {
            create_dir_all(&dir)?;
        }
    }
    File::create(path).map_err(|e| e.context(FileErrorKind::OpenWo(path.to_owned())).into())
}

pub(crate) fn find_path(filename: &'static str, start: &AbsPath) -> FileResult<AbsPathBuf> {
    let mut dir = start.to_owned();
    loop {
        let path = dir.join(filename);
        if std::fs::metadata(&path).is_ok() {
            break Ok(path);
        }
        if !dir.pop() {
            break Err(FileErrorKind::Find {
                filename,
                start: start.to_owned(),
            }
            .into());
        }
    }
}

#[derive(Debug)]
pub(crate) struct LockedFile {
    inner: File,
    path: AbsPathBuf,
}

impl LockedFile {
    pub(crate) fn try_new(path: &AbsPath) -> FileResult<Self> {
        if let Some(parent) = path.parent() {
            create_dir_all(&parent)?;
        }
        let inner = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .with_context(|_| FileErrorKind::OpenRw(path.to_owned()))?;
        inner
            .try_lock_exclusive()
            .map_err(|e| StdError::from(e).context(FileErrorKind::Lock(path.to_owned())))?;
        Ok(Self {
            inner,
            path: path.to_owned(),
        })
    }

    pub(crate) fn path(&self) -> &AbsPath {
        &self.path
    }

    pub(crate) fn is_empty(&self) -> io::Result<bool> {
        self.inner.metadata().map(|m| m.len() == 0)
    }

    pub(crate) fn bincode<T: DeserializeOwned>(&mut self) -> FileResult<T> {
        fn bincode<T: DeserializeOwned>(file: &mut File) -> Fallible<T> {
            file.seek(SeekFrom::Start(0))?;
            bincode::deserialize_from(file).map_err(|e| StdError::from(e).into())
        }

        bincode(&mut self.inner)
            .map_err(|e| e.context(FileErrorKind::Read(self.path.clone())).into())
    }

    pub(crate) fn write_bincode<T: Serialize>(&mut self, value: &T) -> FileResult<()> {
        fn write_bincode<T: Serialize>(file: &mut File, value: &T) -> Fallible<()> {
            file.seek(SeekFrom::Start(0))?;
            file.set_len(0)?;
            bincode::serialize_into(file, value).map_err(|e| StdError::from(e).into())
        }

        write_bincode(&mut self.inner, value)
            .map_err(|e| e.context(FileErrorKind::Write(self.path.clone())).into())
    }
}