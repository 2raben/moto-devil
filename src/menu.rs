//! The player selection screen: "Wer fährt?", the list of players with their
//! progress, and a row for typing a new name. The logic lives in
//! [`crate::flow::select_player`]; this module only draws it.

use bevy::prelude::*;

use crate::flow::{CAN_QUIT, Levels, Screen, Selection};
use crate::hud::{colored_text, text};
use crate::progress::Profiles;
use crate::visuals::palette;

/// The column that holds one row per player.
#[derive(Component)]
struct PlayerList;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::SelectPlayer), spawn_menu)
            .add_systems(
                Update,
                draw_rows.run_if(in_state(Screen::SelectPlayer)).run_if(
                    resource_changed::<Selection>
                        .or_else(resource_changed::<Profiles>)
                        .or_else(any_match_filter::<Added<PlayerList>>),
                ),
            );
    }
}

fn spawn_menu(mut commands: Commands) {
    commands.spawn((
        Name::new("Player selection"),
        DespawnOnExit(Screen::SelectPlayer),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(14.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.08, 0.12, 0.55)),
        children![
            text("MOTO DEVIL", 60.0),
            colored_text("Wer fährt?", 28.0, palette::MUTED),
            (
                PlayerList,
                Node {
                    width: Val::Px(560.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
            ),
            colored_text(
                if CAN_QUIT {
                    "↑/↓ wählen   Enter fahren   Buchstaben tippen = neuer Name   Tab Tasten   Esc beenden"
                } else {
                    "↑/↓ wählen   Enter fahren   Buchstaben tippen = neuer Name   Tab Tasten"
                },
                15.0,
                palette::MUTED,
            ),
        ],
    ));
}

fn row(selected: bool, left: String, right: String) -> impl Bundle {
    let background = if selected {
        palette::HIGHLIGHT
    } else {
        palette::PANEL
    };
    (
        Node {
            width: Val::Percent(100.0),
            padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(background),
        children![text(left, 24.0), text(right, 16.0)],
    )
}

fn draw_rows(
    mut commands: Commands,
    selection: Res<Selection>,
    profiles: Res<Profiles>,
    levels: Res<Levels>,
    lists: Query<Entity, With<PlayerList>>,
) {
    let ids = levels.ids();
    for list in &lists {
        commands.entity(list).despawn_related::<Children>();
        for (index, player) in profiles.players.iter().enumerate() {
            let done = profiles.levels_done(index, &ids);
            let country = levels
                .place(profiles.first_unfinished(index, &ids))
                .map_or("", |(country, ..)| country.name());
            let progress = match done {
                0 => format!("{country} · noch kein Level geschafft"),
                n if n == ids.len() => "alle Länder geschafft ✓".to_string(),
                n => format!("{country} · {n} von {} Leveln geschafft", ids.len()),
            };
            let selected = selection.row == index;
            let marker = if selected { "▶ " } else { "" };
            commands.entity(list).with_child(row(
                selected,
                format!("{marker}{}", player.name),
                progress,
            ));
        }
        let selected = selection.row >= profiles.players.len();
        let typed = if selection.typed.is_empty() && !selected {
            "Namen tippen …".to_string()
        } else {
            format!("{}|", selection.typed)
        };
        commands
            .entity(list)
            .with_child(row(selected, "+ Neuer Spieler".into(), typed));
    }
}
