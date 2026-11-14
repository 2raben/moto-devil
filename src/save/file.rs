//! The native store: one text file per entry in the user's data directory, such
//! as `moto-devil/progress.txt`. Writes go to a temporary file in the same
//! directory first and then replace the old file, so a crash never leaves half
//! a file. See ADR 0005.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::{Entry, Store};

const APP_DIR: &str = "moto-devil";

pub struct FileStore {
    dir: PathBuf,
}

impl FileStore {
    /// Keeps the files in `dir`, which is created on the first write.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The game's folder in the platform's data directory, if there is one.
    pub fn in_data_dir() -> Option<Self> {
        data_dir().map(|dir| Self::new(dir.join(APP_DIR)))
    }

    fn path(&self, entry: Entry) -> PathBuf {
        self.dir.join(format!("{}.txt", entry.name()))
    }
}

impl Store for FileStore {
    fn read(&self, entry: Entry) -> io::Result<Option<String>> {
        read_limited(&self.path(entry), entry)
    }

    fn write(&mut self, entry: Entry, text: &str) -> io::Result<()> {
        write_atomic(&self.path(entry), text)
    }

    fn back_up(&mut self, entry: Entry) -> io::Result<()> {
        let path = self.path(entry);
        let mut backup = path.as_os_str().to_owned();
        backup.push(".broken");
        fs::rename(path, backup)
    }

    fn describe(&self, entry: Entry) -> String {
        self.path(entry).display().to_string()
    }
}

/// The per-user data directory of this platform, without any game subfolder.
fn data_dir() -> Option<PathBuf> {
    let env_dir = |name: &str| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    if cfg!(target_os = "windows") {
        env_dir("APPDATA")
    } else if cfg!(target_os = "macos") {
        env_dir("HOME").map(|home| home.join("Library/Application Support"))
    } else {
        env_dir("XDG_DATA_HOME").or_else(|| env_dir("HOME").map(|home| home.join(".local/share")))
    }
}

/// The file's text, `None` if it does not exist, or an error if it is larger
/// than the entry allows, unreadable, or not UTF-8.
fn read_limited(path: &Path, entry: Entry) -> io::Result<Option<String>> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not a regular file",
        ));
    }
    entry.check_size(usize::try_from(metadata.len()).unwrap_or(usize::MAX))?;
    let bytes = fs::read(path)?;
    entry.check_size(bytes.len())?;
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "not UTF-8 text"))
}

/// Writes `text` to `path` through a temporary file in the same directory.
fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no parent directory"))?;
    fs::create_dir_all(dir)?;
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::{self, KnownLevel, Profiles};
    use crate::save::{load, load_bindings};

    const LEVELS: &[KnownLevel] = &[KnownLevel {
        id: "level-1",
        min_x: 0.0,
        max_x: 2000.0,
    }];

    /// A fresh directory under the system temp dir for one test.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("moto-devil-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn writes_and_reads_back_progress() {
        let dir = scratch("roundtrip");
        let mut store = FileStore::new(dir.join("nested"));
        let mut profiles = Profiles::default();
        let mia = profiles.find_or_add("Mia").unwrap();
        profiles.record_crash(mia, "level-1", 321.0);

        store
            .write(Entry::Progress, &progress::encode(&profiles))
            .unwrap();
        assert_eq!(load(&mut store, LEVELS), profiles);
        assert!(dir.join("nested").join("progress.txt").exists());
        assert!(!dir.join("nested").join("progress.txt.tmp").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_missing_file_is_an_empty_list() {
        let mut store = FileStore::new(scratch("missing"));
        assert_eq!(load(&mut store, LEVELS), Profiles::default());
    }

    #[test]
    fn a_broken_file_is_moved_aside() {
        let dir = scratch("broken");
        fs::create_dir_all(&dir).unwrap();
        let mut store = FileStore::new(&dir);
        let path = dir.join("progress.txt");
        fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        assert_eq!(load(&mut store, LEVELS), Profiles::default());
        assert!(!path.exists());
        assert!(dir.join("progress.txt.broken").exists());

        fs::write(&path, "something else\n").unwrap();
        assert_eq!(load(&mut store, LEVELS), Profiles::default());
        assert!(dir.join("progress.txt.broken").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_broken_keys_file_is_moved_aside() {
        let dir = scratch("keys");
        fs::create_dir_all(&dir).unwrap();
        let mut store = FileStore::new(&dir);
        fs::write(dir.join("keys.txt"), "garbage\n").unwrap();
        assert_eq!(
            load_bindings(&mut store),
            crate::bindings::KeyBindings::default()
        );
        assert!(dir.join("keys.txt.broken").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
