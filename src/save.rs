//! Loads and saves [`Profiles`] and the [`KeyBindings`] through a [`Store`]: files
//! in the user's data directory natively, the page's local storage in the
//! browser, and memory in tests. See ADR 0005 and ADR 0010.
//!
//! Every stored text is untrusted: stores refuse texts larger than the entry's
//! limit, and [`progress::decode`] or [`bindings::decode`] check the rest line
//! by line. An unusable entry is moved aside as a backup and the game starts
//! with an empty list or the default keys.
//!
//! This plugin is not part of the headless simulation. `main.rs` chooses the
//! store for the platform and inserts it as [`SaveStore`]; without one the game
//! runs but does not save.

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(not(target_arch = "wasm32"))]
mod file;

use std::io;

use bevy::prelude::*;

use crate::bindings::{self, KeyBindings, MAX_KEYS_BYTES};
use crate::flow::Levels;
use crate::progress::{self, KnownLevel, MAX_SAVE_BYTES, Profiles};

#[cfg(target_arch = "wasm32")]
pub use browser::BrowserStore;
#[cfg(not(target_arch = "wasm32"))]
pub use file::FileStore;

/// One saved text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    /// The players and their progress, in the format of [`progress::encode`].
    Progress,
    /// The riding keys, in the format of [`bindings::encode`].
    Keys,
}

impl Entry {
    /// The stable name every store derives its file or key from.
    pub fn name(self) -> &'static str {
        match self {
            Self::Progress => "progress",
            Self::Keys => "keys",
        }
    }

    /// Larger texts are not read.
    pub fn max_bytes(self) -> usize {
        match self {
            Self::Progress => MAX_SAVE_BYTES,
            Self::Keys => MAX_KEYS_BYTES,
        }
    }

    /// An error if `len` bytes are more than this entry may hold.
    fn check_size(self, len: usize) -> io::Result<()> {
        if len > self.max_bytes() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("larger than {} bytes", self.max_bytes()),
            ));
        }
        Ok(())
    }
}

/// Where saved texts live. A store only moves text; formats and checks stay
/// with the data they describe.
pub trait Store: Send + Sync + 'static {
    /// The entry's text, `None` if there is none, or an error if it cannot be
    /// read or is larger than [`Entry::max_bytes`].
    fn read(&self, entry: Entry) -> io::Result<Option<String>>;

    /// Replaces the entry's text. A failed write leaves the old text intact.
    fn write(&mut self, entry: Entry, text: &str) -> io::Result<()>;

    /// Moves an unusable entry aside, so it is kept but no longer read.
    fn back_up(&mut self, entry: Entry) -> io::Result<()>;

    /// Where the entry lives, for log messages.
    fn describe(&self, entry: Entry) -> String;
}

/// The store this run saves to.
#[derive(Resource)]
pub struct SaveStore(Box<dyn Store>);

impl SaveStore {
    pub fn new(store: impl Store) -> Self {
        Self(Box::new(store))
    }
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_profiles, load_keys))
            .add_systems(
                Last,
                (
                    save_profiles.run_if(resource_changed::<Profiles>),
                    save_keys.run_if(resource_changed::<KeyBindings>),
                )
                    .run_if(resource_exists::<SaveStore>),
            );
    }
}

/// The progress text most recently loaded or written, so unchanged profiles
/// are not written again.
#[derive(Resource, Default)]
struct SavedText(String);

/// The same for the key bindings.
#[derive(Resource, Default)]
struct SavedKeys(String);

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

/// Reads the profiles. A missing entry gives an empty list; an entry that
/// cannot be used is moved aside and also gives an empty list.
pub fn load(store: &mut dyn Store, levels: &[KnownLevel]) -> Profiles {
    load_with(store, Entry::Progress, |text| {
        progress::decode(text, levels)
    })
}

/// Reads the key bindings. A missing entry gives the default keys; an entry
/// that cannot be used is moved aside and also gives them.
pub fn load_bindings(store: &mut dyn Store) -> KeyBindings {
    load_with(store, Entry::Keys, bindings::decode)
}

/// Decodes the entry, or gives the default if it is missing or cannot be
/// used, moving a broken entry aside.
fn load_with<T: Default, E: std::fmt::Display>(
    store: &mut dyn Store,
    entry: Entry,
    decode: impl FnOnce(&str) -> Result<T, E>,
) -> T {
    let problem = match store.read(entry) {
        Ok(None) => return T::default(),
        Ok(Some(text)) => match decode(&text) {
            Ok(value) => return value,
            Err(error) => format!("Ignoring {}: {error}", store.describe(entry)),
        },
        Err(error) => format!("Could not read {}: {error}", store.describe(entry)),
    };
    warn!("{problem}");
    if let Err(error) = store.back_up(entry) {
        warn!("Could not move {} aside: {error}", store.describe(entry));
    }
    T::default()
}

fn load_profiles(mut commands: Commands, levels: Res<Levels>, store: Option<ResMut<SaveStore>>) {
    let Some(mut store) = store else {
        warn!("No place to save found; progress will not be saved");
        return;
    };
    let profiles = load(store.0.as_mut(), &known_levels(&levels));
    commands.insert_resource(SavedText(progress::encode(&profiles)));
    commands.insert_resource(profiles);
}

fn save_profiles(
    profiles: Res<Profiles>,
    saved: Option<ResMut<SavedText>>,
    mut store: ResMut<SaveStore>,
) {
    if let Some(mut saved) = saved {
        let text = progress::encode(&profiles);
        write_if_changed(store.0.as_mut(), Entry::Progress, &mut saved.0, text);
    }
}

fn load_keys(mut commands: Commands, store: Option<ResMut<SaveStore>>) {
    let Some(mut store) = store else {
        return;
    };
    let bindings = load_bindings(store.0.as_mut());
    commands.insert_resource(SavedKeys(bindings::encode(&bindings)));
    commands.insert_resource(bindings);
}

fn save_keys(
    bindings: Res<KeyBindings>,
    saved: Option<ResMut<SavedKeys>>,
    mut store: ResMut<SaveStore>,
) {
    if let Some(mut saved) = saved {
        let text = bindings::encode(&bindings);
        write_if_changed(store.0.as_mut(), Entry::Keys, &mut saved.0, text);
    }
}

/// Writes `text` unless it is what was last loaded or written.
fn write_if_changed(store: &mut dyn Store, entry: Entry, saved: &mut String, text: String) {
    if text == *saved {
        return;
    }
    match store.write(entry, &text) {
        Ok(()) => *saved = text,
        Err(error) => warn!("Could not save {}: {error}", store.describe(entry)),
    }
}

/// A store in memory, shared between clones so a test can look inside.
#[cfg(test)]
#[derive(Clone, Default)]
pub struct MemoryStore(
    std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<String, String>>>,
);

#[cfg(test)]
impl MemoryStore {
    /// The text stored under `key`: an entry name, or a name with `.broken`.
    pub fn get(&self, key: &str) -> Option<String> {
        self.0.lock().unwrap().get(key).cloned()
    }

    pub fn set(&self, key: &str, text: &str) {
        self.0
            .lock()
            .unwrap()
            .insert(key.to_owned(), text.to_owned());
    }
}

#[cfg(test)]
impl Store for MemoryStore {
    fn read(&self, entry: Entry) -> io::Result<Option<String>> {
        let text = self.get(entry.name());
        if let Some(text) = &text {
            entry.check_size(text.len())?;
        }
        Ok(text)
    }

    fn write(&mut self, entry: Entry, text: &str) -> io::Result<()> {
        self.set(entry.name(), text);
        Ok(())
    }

    fn back_up(&mut self, entry: Entry) -> io::Result<()> {
        let mut entries = self.0.lock().unwrap();
        if let Some(text) = entries.remove(entry.name()) {
            entries.insert(format!("{}.broken", entry.name()), text);
        }
        Ok(())
    }

    fn describe(&self, entry: Entry) -> String {
        format!("memory entry `{}`", entry.name())
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

    #[test]
    fn writes_and_reads_back_progress() {
        let mut store = MemoryStore::default();
        let mut profiles = Profiles::default();
        let mia = profiles.find_or_add("Mia").unwrap();
        profiles.record_crash(mia, "level-1", 321.0);

        let mut saved = String::new();
        write_if_changed(
            &mut store,
            Entry::Progress,
            &mut saved,
            progress::encode(&profiles),
        );
        assert_eq!(load(&mut store, LEVELS), profiles);
        assert_eq!(saved, progress::encode(&profiles));
    }

    #[test]
    fn a_missing_entry_is_an_empty_list() {
        assert_eq!(
            load(&mut MemoryStore::default(), LEVELS),
            Profiles::default()
        );
    }

    #[test]
    fn a_broken_entry_is_moved_aside() {
        let mut store = MemoryStore::default();
        store.set("progress", "something else\n");
        assert_eq!(load(&mut store, LEVELS), Profiles::default());
        assert_eq!(store.get("progress"), None);
        assert_eq!(
            store.get("progress.broken").as_deref(),
            Some("something else\n")
        );
    }

    #[test]
    fn an_oversized_entry_is_moved_aside() {
        let mut store = MemoryStore::default();
        let huge = format!("moto-devil-save 1\n{}", "x".repeat(MAX_SAVE_BYTES));
        store.set("progress", &huge);
        assert_eq!(load(&mut store, LEVELS), Profiles::default());
        assert!(store.get("progress.broken").is_some());
    }

    #[test]
    fn key_bindings_round_trip_and_fall_back_to_the_defaults() {
        let mut store = MemoryStore::default();
        assert_eq!(load_bindings(&mut store), KeyBindings::default(), "none");

        let keys = bindings::decode("moto-devil-keys 1\nkey\tjump\tKeyK\tK\n").unwrap();
        assert_ne!(keys, KeyBindings::default());
        store.write(Entry::Keys, &bindings::encode(&keys)).unwrap();
        assert_eq!(load_bindings(&mut store), keys);

        store.set("keys", "garbage\n");
        assert_eq!(load_bindings(&mut store), KeyBindings::default());
        assert_eq!(store.get("keys.broken").as_deref(), Some("garbage\n"));
    }

    /// An app with only the save plugin and the resources it reads.
    fn app(store: Option<MemoryStore>) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Levels(crate::levels::all()))
            .init_resource::<Profiles>()
            .init_resource::<KeyBindings>()
            .add_plugins(SavePlugin);
        if let Some(store) = store {
            app.insert_resource(SaveStore::new(store));
        }
        app
    }

    #[test]
    fn the_plugin_loads_at_startup_and_saves_changes() {
        let store = MemoryStore::default();
        store.set("progress", "moto-devil-save 1\nplayer\tMia\n");
        let mut app = app(Some(store.clone()));
        app.update();
        assert_eq!(app.world().resource::<Profiles>().players.len(), 1);
        assert_eq!(store.get("keys"), None, "nothing changed, nothing written");

        app.world_mut()
            .resource_mut::<Profiles>()
            .find_or_add("Ben")
            .unwrap();
        app.update();
        assert_eq!(
            store.get("progress").as_deref(),
            Some("moto-devil-save 1\nplayer\tMia\nplayer\tBen\n")
        );
    }

    #[test]
    fn without_a_store_the_game_runs_and_does_not_save() {
        let mut app = app(None);
        app.update();
        app.world_mut()
            .resource_mut::<Profiles>()
            .find_or_add("Mia")
            .unwrap();
        app.update();
        assert_eq!(app.world().resource::<Profiles>().players.len(), 1);
    }
}
