//! Players and their progress: name rules, gravestones, and the save format.
//!
//! Everything here is pure data and functions. Reading and writing the file is
//! done by [`crate::save`]; see ADR 0005 for the format and its limits.

use std::collections::BTreeMap;

use bevy::prelude::*;

/// A name has at most this many characters.
pub const MAX_NAME_CHARS: usize = 12;
/// At most this many players are kept.
pub const MAX_PLAYERS: usize = 32;
/// Larger save files are not read.
pub const MAX_SAVE_BYTES: usize = 64 * 1024;
/// The first line of a save file is this word followed by the format version.
pub const SAVE_HEADER: &str = "moto-devil-save";
pub const SAVE_VERSION: u32 = 1;

/// One player's progress in one level.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LevelProgress {
    pub done: bool,
    /// How far right the player has crashed, in world units.
    pub farthest_crash: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub name: String,
    /// Progress by level id.
    pub levels: BTreeMap<String, LevelProgress>,
}

impl Player {
    pub fn new(name: String) -> Self {
        Self {
            name,
            levels: BTreeMap::new(),
        }
    }

    pub fn level(&self, id: &str) -> Option<&LevelProgress> {
        self.levels.get(id)
    }

    pub fn is_done(&self, id: &str) -> bool {
        self.level(id).is_some_and(|level| level.done)
    }
}

/// Where a player's gravestone stands in a level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Grave {
    /// At the farthest crash, measured along x.
    Crash(f32),
    /// At the finish, with a tick.
    Finished,
}

/// Everyone who has played on this computer.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct Profiles {
    pub players: Vec<Player>,
}

/// The index into [`Profiles::players`] of the player who is riding.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivePlayer(pub Option<usize>);

/// Letters (including German umlauts and other Latin letters), digits, and spaces.
pub fn is_name_char(c: char) -> bool {
    let latin_letter = c.is_alphabetic() && (c.is_ascii() || ('\u{C0}'..='\u{17F}').contains(&c));
    latin_letter || c.is_ascii_digit() || c == ' '
}

/// The name as it will be stored, or `None` if it breaks the rules: 1 to
/// [`MAX_NAME_CHARS`] allowed characters after trimming, single spaces inside.
pub fn clean_name(raw: &str) -> Option<String> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let length = name.chars().count();
    let valid = (1..=MAX_NAME_CHARS).contains(&length) && name.chars().all(is_name_char);
    valid.then_some(name)
}

fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

impl Profiles {
    /// The player with this name, ignoring upper and lower case.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.players
            .iter()
            .position(|player| same_name(&player.name, name))
    }

    /// The existing player with this name, or a new one. `None` if the name is
    /// invalid or the list is full.
    pub fn find_or_add(&mut self, raw: &str) -> Option<usize> {
        let name = clean_name(raw)?;
        if let Some(index) = self.find(&name) {
            return Some(index);
        }
        if self.players.len() >= MAX_PLAYERS {
            return None;
        }
        self.players.push(Player::new(name));
        Some(self.players.len() - 1)
    }

    /// Moves the gravestone forward if `x` is farther than any earlier crash.
    /// Returns whether anything changed. A finished level keeps its stone at
    /// the finish.
    pub fn record_crash(&mut self, player: usize, level: &str, x: f32) -> bool {
        let Some(player) = self.players.get_mut(player) else {
            return false;
        };
        let progress = player.levels.entry(level.to_owned()).or_default();
        if progress.done
            || progress
                .farthest_crash
                .is_some_and(|farthest| farthest >= x)
        {
            return false;
        }
        progress.farthest_crash = Some(x);
        true
    }

    /// Marks the level as done. Returns whether this is new.
    pub fn record_finish(&mut self, player: usize, level: &str) -> bool {
        let Some(player) = self.players.get_mut(player) else {
            return false;
        };
        let progress = player.levels.entry(level.to_owned()).or_default();
        let new = !progress.done;
        progress.done = true;
        new
    }

    /// How many of `levels` the player has finished.
    pub fn levels_done(&self, player: usize, levels: &[&str]) -> usize {
        self.players.get(player).map_or(0, |player| {
            levels.iter().filter(|id| player.is_done(id)).count()
        })
    }

    /// The index of the first level the player has not finished, or 0 when all
    /// are done.
    pub fn first_unfinished(&self, player: usize, levels: &[&str]) -> usize {
        let Some(player) = self.players.get(player) else {
            return 0;
        };
        levels
            .iter()
            .position(|id| !player.is_done(id))
            .unwrap_or(0)
    }

    /// Where each player's gravestone stands in a level, for players who have one.
    pub fn graves<'a>(&'a self, level: &'a str) -> impl Iterator<Item = (usize, Grave)> + 'a {
        self.players
            .iter()
            .enumerate()
            .filter_map(move |(index, player)| {
                let progress = player.level(level)?;
                if progress.done {
                    Some((index, Grave::Finished))
                } else {
                    progress.farthest_crash.map(|x| (index, Grave::Crash(x)))
                }
            })
    }
}

/// A level the save file may mention, with the x range its crashes must lie in.
#[derive(Clone, Copy, Debug)]
pub struct KnownLevel<'a> {
    pub id: &'a str,
    pub min_x: f32,
    pub max_x: f32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SaveError {
    TooLarge,
    MissingHeader,
    UnsupportedVersion(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(f, "the save file is larger than {MAX_SAVE_BYTES} bytes"),
            Self::MissingHeader => write!(f, "the save file does not start with `{SAVE_HEADER}`"),
            Self::UnsupportedVersion(version) => {
                write!(f, "save format version `{version}` is not supported")
            }
        }
    }
}

/// Writes the profiles in the line-based save format. The first line names the
/// format and its version; then each player line is followed by that player's
/// level lines. Fields are separated by a tab, which names cannot contain:
///
/// ```text
/// moto-devil-save 1
/// player<TAB>Mia
/// level<TAB>level-1<TAB>done
/// level<TAB>level-2<TAB>crash<TAB>1234.5
/// ```
pub fn encode(profiles: &Profiles) -> String {
    let mut out = format!("{SAVE_HEADER} {SAVE_VERSION}\n");
    for player in &profiles.players {
        out.push_str(&format!("player\t{}\n", player.name));
        for (id, progress) in &player.levels {
            if progress.done {
                out.push_str(&format!("level\t{id}\tdone\n"));
            }
            if let Some(x) = progress.farthest_crash {
                out.push_str(&format!("level\t{id}\tcrash\t{x:.1}\n"));
            }
        }
    }
    out
}

/// Reads a save file. The header must match; after that, lines that break a
/// rule are skipped, so one damaged line does not lose everyone's progress.
/// Names are checked like typed names, unknown levels are dropped, and crash
/// positions must be finite and inside the level.
pub fn decode(text: &str, levels: &[KnownLevel]) -> Result<Profiles, SaveError> {
    if text.len() > MAX_SAVE_BYTES {
        return Err(SaveError::TooLarge);
    }
    let mut lines = text.lines();
    let header = lines.next().ok_or(SaveError::MissingHeader)?;
    let version = header
        .strip_prefix(SAVE_HEADER)
        .and_then(|rest| rest.strip_prefix(' '))
        .ok_or(SaveError::MissingHeader)?;
    if version.trim() != SAVE_VERSION.to_string() {
        return Err(SaveError::UnsupportedVersion(
            version.trim().chars().take(16).collect(),
        ));
    }

    let mut profiles = Profiles::default();
    // The player that `level` lines belong to; `None` after a skipped player.
    let mut current: Option<usize> = None;
    for line in lines {
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            ["player", name] => {
                current = None;
                let Some(name) = clean_name(name) else {
                    continue;
                };
                if profiles.find(&name).is_some() || profiles.players.len() >= MAX_PLAYERS {
                    continue;
                }
                profiles.players.push(Player::new(name));
                current = Some(profiles.players.len() - 1);
            }
            ["level", id, rest @ ..] => {
                let Some(player) = current else { continue };
                let Some(known) = levels.iter().find(|level| level.id == *id) else {
                    continue;
                };
                let progress = profiles.players[player]
                    .levels
                    .entry(known.id.to_owned())
                    .or_default();
                match rest {
                    ["done"] => progress.done = true,
                    ["crash", x] => {
                        if let Ok(x) = x.parse::<f32>()
                            && x.is_finite()
                            && (known.min_x..=known.max_x).contains(&x)
                        {
                            progress.farthest_crash = Some(x);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    // Drop empty entries left behind by skipped lines.
    for player in &mut profiles.players {
        player
            .levels
            .retain(|_, progress| progress.done || progress.farthest_crash.is_some());
    }
    Ok(profiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEVELS: &[KnownLevel] = &[
        KnownLevel {
            id: "level-1",
            min_x: 0.0,
            max_x: 2000.0,
        },
        KnownLevel {
            id: "level-2",
            min_x: 0.0,
            max_x: 3000.0,
        },
    ];
    const IDS: &[&str] = &["level-1", "level-2"];

    #[test]
    fn names_follow_the_rules() {
        assert_eq!(clean_name("  Mia  "), Some("Mia".into()));
        assert_eq!(clean_name("Jürgen 2"), Some("Jürgen 2".into()));
        assert_eq!(clean_name("Anna   Lena"), Some("Anna Lena".into()));
        assert_eq!(clean_name(""), None);
        assert_eq!(clean_name("   "), None);
        assert_eq!(clean_name("ABCDEFGHIJKLM"), None, "13 characters");
        assert_eq!(clean_name("Tab\there"), Some("Tab here".into()));
        assert_eq!(clean_name("Mia!"), None);
        assert_eq!(clean_name("日本"), None, "the font has no CJK glyphs");
    }

    #[test]
    fn an_existing_name_selects_that_player() {
        let mut profiles = Profiles::default();
        assert_eq!(profiles.find_or_add("Mia"), Some(0));
        assert_eq!(profiles.find_or_add("ben"), Some(1));
        assert_eq!(profiles.find_or_add("MIA"), Some(0));
        assert_eq!(profiles.players.len(), 2);
        assert_eq!(profiles.find_or_add("!"), None);
    }

    #[test]
    fn the_player_list_is_capped() {
        let mut profiles = Profiles::default();
        for i in 0..MAX_PLAYERS {
            assert!(profiles.find_or_add(&format!("P{i}")).is_some());
        }
        assert_eq!(profiles.find_or_add("One more"), None);
    }

    #[test]
    fn gravestones_only_move_forward_and_stay_at_the_finish() {
        let mut profiles = Profiles::default();
        let mia = profiles.find_or_add("Mia").unwrap();
        assert!(profiles.record_crash(mia, "level-1", 500.0));
        assert!(!profiles.record_crash(mia, "level-1", 300.0));
        assert!(profiles.record_crash(mia, "level-1", 800.0));
        assert_eq!(
            profiles.graves("level-1").collect::<Vec<_>>(),
            vec![(mia, Grave::Crash(800.0))]
        );

        assert!(profiles.record_finish(mia, "level-1"));
        assert!(!profiles.record_finish(mia, "level-1"));
        assert!(!profiles.record_crash(mia, "level-1", 1500.0));
        assert_eq!(
            profiles.graves("level-1").collect::<Vec<_>>(),
            vec![(mia, Grave::Finished)]
        );
        assert_eq!(profiles.graves("level-2").count(), 0);
    }

    #[test]
    fn players_continue_at_their_first_unfinished_level() {
        let mut profiles = Profiles::default();
        let mia = profiles.find_or_add("Mia").unwrap();
        assert_eq!(profiles.first_unfinished(mia, IDS), 0);
        profiles.record_finish(mia, "level-1");
        assert_eq!(profiles.first_unfinished(mia, IDS), 1);
        assert_eq!(profiles.levels_done(mia, IDS), 1);
        profiles.record_finish(mia, "level-2");
        assert_eq!(
            profiles.first_unfinished(mia, IDS),
            0,
            "all done: start again"
        );
    }

    #[test]
    fn the_save_format_round_trips() {
        let mut profiles = Profiles::default();
        let mia = profiles.find_or_add("Mia").unwrap();
        let jurgen = profiles.find_or_add("Jürgen").unwrap();
        profiles.record_finish(mia, "level-1");
        profiles.record_crash(mia, "level-2", 1234.5);
        profiles.record_crash(jurgen, "level-1", 99.0);
        let text = encode(&profiles);
        assert!(text.starts_with("moto-devil-save 1\n"));
        assert_eq!(decode(&text, LEVELS), Ok(profiles));
    }

    #[test]
    fn damaged_lines_are_skipped() {
        let text = "moto-devil-save 1\n\
            player\tMia\n\
            level\tlevel-1\tcrash\tNaN\n\
            level\tlevel-1\tcrash\t99999\n\
            level\tlevel-9\tdone\n\
            level\tlevel-2\tdone\n\
            nonsense\n\
            player\tBad!Name\n\
            level\tlevel-1\tdone\n\
            player\tmia\n\
            player\tBen\n\
            level\tlevel-1\tcrash\t42\n";
        let profiles = decode(text, LEVELS).unwrap();
        let names: Vec<_> = profiles.players.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Mia", "Ben"]);
        let mia = &profiles.players[0];
        assert!(mia.is_done("level-2"));
        assert!(
            mia.level("level-1").is_none(),
            "invalid crashes are dropped"
        );
        assert!(mia.level("level-9").is_none());
        assert_eq!(
            profiles.players[1].level("level-1").unwrap().farthest_crash,
            Some(42.0)
        );
    }

    #[test]
    fn foreign_or_oversized_files_are_rejected() {
        assert_eq!(decode("", LEVELS), Err(SaveError::MissingHeader));
        assert_eq!(decode("hello\n", LEVELS), Err(SaveError::MissingHeader));
        assert_eq!(
            decode("moto-devil-save 2\n", LEVELS),
            Err(SaveError::UnsupportedVersion("2".into()))
        );
        let huge = format!("moto-devil-save 1\n{}", "x".repeat(MAX_SAVE_BYTES));
        assert_eq!(decode(&huge, LEVELS), Err(SaveError::TooLarge));
    }
}
