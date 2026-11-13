//! Game flow: choosing a player, loading levels, and moving between them.
//!
//! Menu keys arrive as [`MenuAction`]s in [`MenuInput`], written by the input
//! layer, so the flow runs headless in tests like the rest of the simulation
//! (ADR 0006).

use bevy::prelude::*;

use crate::bike::{BikeTuning, Controls};
use crate::course::{self, LevelEntity};
use crate::level::{Country, Level};
use crate::progress::{self, ActivePlayer, MAX_NAME_CHARS, Profiles};
use crate::run::Run;

/// Which screen is showing.
#[derive(States, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Screen {
    /// "Wer fährt?": pick or type a name.
    #[default]
    SelectPlayer,
    /// Riding a level.
    Playing,
    /// "Tasten": choosing the riding keys. The editor lives in the input layer
    /// (`bindings.rs`); the flow only leaves this screen again.
    Settings,
}

/// All levels in play order.
#[derive(Resource, Clone, Debug)]
pub struct Levels(pub Vec<Level>);

impl Default for Levels {
    fn default() -> Self {
        let levels = crate::levels::all();
        for level in &levels {
            debug_assert_eq!(level.validate(), Ok(()), "level {}", level.id);
        }
        Self(levels)
    }
}

impl Levels {
    pub fn ids(&self) -> Vec<&'static str> {
        self.0.iter().map(|level| level.id).collect()
    }

    /// Where the level at `index` stands in its country: the country, the
    /// level's number in it counted from 1, and how many levels it has.
    pub fn place(&self, index: usize) -> Option<(Country, usize, usize)> {
        let country = self.0.get(index)?.country;
        let same = |level: &&Level| level.country == country;
        let first = self.0[..=index]
            .iter()
            .rposition(|level| level.country != country)
            .map_or(0, |i| i + 1);
        let count = self.0[first..].iter().take_while(same).count();
        Some((country, index - first + 1, count))
    }

    /// The country the next level belongs to, when the level at `index` is
    /// the last one of its country and another country follows.
    pub fn next_country(&self, index: usize) -> Option<Country> {
        let here = self.0.get(index)?.country;
        let next = self.0.get(index + 1)?.country;
        (next != here).then_some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_know_their_place_in_a_country() {
        let level = |id, country| Level {
            id,
            country,
            ..crate::levels::level_one()
        };
        let levels = Levels(vec![
            level("a", Country::Training),
            level("b", Country::Egypt),
            level("c", Country::Egypt),
            level("d", Country::Paris),
        ]);
        assert_eq!(levels.place(0), Some((Country::Training, 1, 1)));
        assert_eq!(levels.place(1), Some((Country::Egypt, 1, 2)));
        assert_eq!(levels.place(2), Some((Country::Egypt, 2, 2)));
        assert_eq!(levels.place(4), None);
        assert_eq!(levels.next_country(0), Some(Country::Egypt));
        assert_eq!(levels.next_country(1), None);
        assert_eq!(levels.next_country(2), Some(Country::Paris));
        assert_eq!(levels.next_country(3), None, "the last level");
    }
}

/// The index into [`Levels`] of the level being played.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CurrentLevel(pub usize);

/// A menu key, independent of the device that produced it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Up,
    Down,
    /// Enter: choose the player, or go on after the finish.
    Confirm,
    /// Escape: back to the player selection, or quit from there.
    Back,
    /// A typed character for a new name.
    Type(char),
    /// Backspace while typing a name.
    Erase,
    /// Tab on the player selection: open the key settings.
    Settings,
}

/// Menu keys since the flow last looked, oldest first.
#[derive(Resource, Clone, Debug, Default)]
pub struct MenuInput(pub Vec<MenuAction>);

/// The player selection: the highlighted row and the name being typed. Rows
/// `0..players.len()` are players; the last row is "new player".
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct Selection {
    pub row: usize,
    pub typed: String,
}

/// Handles menu keys on the player selection screen.
#[allow(clippy::too_many_arguments)]
pub fn select_player(
    mut input: ResMut<MenuInput>,
    mut selection: ResMut<Selection>,
    mut profiles: ResMut<Profiles>,
    mut active: ResMut<ActivePlayer>,
    mut current: ResMut<CurrentLevel>,
    levels: Res<Levels>,
    mut next: ResMut<NextState<Screen>>,
    mut exit: MessageWriter<AppExit>,
) {
    let new_row = profiles.players.len();
    selection.row = selection.row.min(new_row);
    for action in std::mem::take(&mut input.0) {
        match action {
            MenuAction::Up => selection.row = selection.row.saturating_sub(1),
            MenuAction::Down => selection.row = (selection.row + 1).min(new_row),
            MenuAction::Type(c) => {
                let fits = selection.typed.chars().count() < MAX_NAME_CHARS;
                let leading_space = c == ' ' && selection.typed.is_empty();
                if progress::is_name_char(c) && fits && !leading_space {
                    selection.typed.push(c);
                }
                selection.row = new_row;
            }
            MenuAction::Erase => {
                selection.typed.pop();
                selection.row = new_row;
            }
            MenuAction::Confirm => {
                let player = if selection.row < new_row {
                    Some(selection.row)
                } else {
                    profiles.find_or_add(&selection.typed)
                };
                let Some(player) = player else {
                    continue;
                };
                active.0 = Some(player);
                current.0 = profiles.first_unfinished(player, &levels.ids());
                selection.typed.clear();
                selection.row = player;
                next.set(Screen::Playing);
                return;
            }
            MenuAction::Settings => {
                next.set(Screen::Settings);
                return;
            }
            MenuAction::Back => {
                exit.write(AppExit::Success);
                return;
            }
        }
    }
}

/// Handles menu keys on the key settings: Escape goes back to the player
/// selection. The settings themselves are edited by the input layer.
pub fn settings_menu(mut input: ResMut<MenuInput>, mut next: ResMut<NextState<Screen>>) {
    for action in std::mem::take(&mut input.0) {
        if action == MenuAction::Back {
            next.set(Screen::SelectPlayer);
            return;
        }
    }
}

/// Handles menu keys while riding: Enter after the finish goes to the next
/// level, Escape goes back to the player selection.
pub fn playing_menu(
    mut commands: Commands,
    mut input: ResMut<MenuInput>,
    run: Res<Run>,
    levels: Res<Levels>,
    mut current: ResMut<CurrentLevel>,
    mut next: ResMut<NextState<Screen>>,
) {
    for action in std::mem::take(&mut input.0) {
        match action {
            MenuAction::Confirm if run.finished => {
                if current.0 + 1 < levels.0.len() {
                    current.0 += 1;
                    commands.run_system_cached(load_level);
                } else {
                    next.set(Screen::SelectPlayer);
                }
                return;
            }
            MenuAction::Back => {
                next.set(Screen::SelectPlayer);
                return;
            }
            _ => {}
        }
    }
}

/// Replaces whatever level is loaded with [`CurrentLevel`].
pub fn load_level(
    mut commands: Commands,
    levels: Res<Levels>,
    current: Res<CurrentLevel>,
    tuning: Res<BikeTuning>,
    old: Query<Entity, With<LevelEntity>>,
) {
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let Some(level) = levels.0.get(current.0) else {
        return;
    };
    commands.insert_resource(level.clone());
    commands.insert_resource(Run::new(level.start));
    commands.insert_resource(Controls::default());
    course::spawn_course(&mut commands, level, &tuning);
}

/// Removes the level when leaving the riding screen.
pub fn unload_level(mut commands: Commands, old: Query<Entity, With<LevelEntity>>) {
    for entity in &old {
        commands.entity(entity).despawn();
    }
}
