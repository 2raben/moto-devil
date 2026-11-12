//! Reads and writes [`Profiles`] in the user's data directory.
//!
//! The file is untrusted: it is size-limited and checked line by line by
//! [`progress::decode`]. An unreadable file is moved aside as a backup and the
//! game starts with an empty list. Writes go to a temporary file in the same
//! directory first and then replace the old file, so a crash never leaves half a
//! file. See ADR 0005.
//!
//! This plugin is not part of the headless simulation; tests use the pure
//! functions and a temporary directory.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use bevy::prelude::*;

use crate::flow::Levels;
use crate::progress::{self, KnownLevel, MAX_SAVE_BYTES, Profiles};

const APP_DIR: &str = "moto-devil";
const FILE_NAME: &str = "progress.txt";

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_profiles)
            .add_systems(Last, save_profiles.run_if(resource_changed::<Profiles>));
    }
}

/// The text most recently loaded or written, so unchanged profiles are not
/// written again.
#[derive(Resource, Default)]
struct SavedText(String);

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

/// Where the progress file lives, if the platform has a data directory.
pub fn save_path() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join(APP_DIR).join(FILE_NAME))
}

fn known_levels(levels: &Levels) -> Vec<KnownLevel<'_>> {
    levels
        .0
        .iter()
        .map(|level| {
            let (min_x, max_x) = level.x_range();
            KnownLevel {
                id: level.id,
                min_x,
                max_x,
            }
        })
        .collect()
}

/// Reads the profiles at `path`. A missing file gives an empty list; a file that
/// cannot be used is renamed to `<name>.broken` and also gives an empty list.
pub fn load(path: &Path, levels: &[KnownLevel]) -> Profiles {
    let text = match read_limited(path) {
        Ok(Some(text)) => text,
        Ok(None) => return Profiles::default(),
        Err(error) => {
            warn!("Could not read {}: {error}", path.display());
            back_up(path);
            return Profiles::default();
        }
    };
    match progress::decode(&text, levels) {
        Ok(profiles) => profiles,
        Err(error) => {
            warn!("Ignoring {}: {error}", path.display());
            back_up(path);
            Profiles::default()
        }
    }
}

/// The file's text, `None` if it does not exist, or an error if it is too
/// large, unreadable, or not UTF-8.
fn read_limited(path: &Path) -> io::Result<Option<String>> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() || metadata.len() > MAX_SAVE_BYTES as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not a regular file of a sensible size",
        ));
    }
    let bytes = fs::read(path)?;
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "not UTF-8 text"))
}

fn back_up(path: &Path) {
    let mut backup = path.as_os_str().to_owned();
    backup.push(".broken");
    if let Err(error) = fs::rename(path, &backup) {
        warn!("Could not move {} aside: {error}", path.display());
    }
}

/// Writes `text` to `path` through a temporary file in the same directory.
pub fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
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

fn load_profiles(mut commands: Commands, levels: Res<Levels>) {
    let Some(path) = save_path() else {
        warn!("No data directory found; progress will not be saved");
        return;
    };
    let profiles = load(&path, &known_levels(&levels));
    commands.insert_resource(SavedText(progress::encode(&profiles)));
    commands.insert_resource(profiles);
}

fn save_profiles(profiles: Res<Profiles>, saved: Option<ResMut<SavedText>>) {
    let Some(mut saved) = saved else {
        return;
    };
    let text = progress::encode(&profiles);
    if text == saved.0 {
        return;
    }
    let Some(path) = save_path() else {
        return;
    };
    match write_atomic(&path, &text) {
        Ok(()) => saved.0 = text,
        Err(error) => warn!("Could not save progress to {}: {error}", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let path = dir.join("nested").join(FILE_NAME);
        let mut profiles = Profiles::default();
        let mia = profiles.find_or_add("Mia").unwrap();
        profiles.record_crash(mia, "level-1", 321.0);

        write_atomic(&path, &progress::encode(&profiles)).unwrap();
        assert_eq!(load(&path, LEVELS), profiles);
        assert!(!dir.join("nested").join("progress.txt.tmp").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_missing_file_is_an_empty_list() {
        let dir = scratch("missing");
        assert_eq!(load(&dir.join(FILE_NAME), LEVELS), Profiles::default());
    }

    #[test]
    fn a_broken_file_is_moved_aside() {
        let dir = scratch("broken");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE_NAME);
        fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        assert_eq!(load(&path, LEVELS), Profiles::default());
        assert!(!path.exists());
        assert!(dir.join("progress.txt.broken").exists());

        fs::write(&path, "something else\n").unwrap();
        assert_eq!(load(&path, LEVELS), Profiles::default());
        assert!(dir.join("progress.txt.broken").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
