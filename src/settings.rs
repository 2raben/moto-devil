//! The key settings screen: "Tasten", one row per riding action with its two
//! key slots, and a row that restores the defaults. The editing lives in
//! [`crate::bindings`]; this module only draws it.

use bevy::prelude::*;

use crate::bindings::{Action, BindingEditor, KeyBindings, RESET_ROW, SLOTS};
use crate::flow::Screen;
use crate::hud::{colored_text, text};
use crate::visuals::palette;

/// The column that holds one row per action.
#[derive(Component)]
struct BindingList;

/// The line under the list: what to do next, or what the last change did.
#[derive(Component)]
struct EditorHint;

pub struct SettingsViewPlugin;

impl Plugin for SettingsViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::Settings), spawn_settings)
            .add_systems(
                Update,
                draw_settings.run_if(in_state(Screen::Settings)).run_if(
                    resource_changed::<BindingEditor>
                        .or_else(resource_changed::<KeyBindings>)
                        .or_else(any_match_filter::<Added<BindingList>>),
                ),
            );
    }
}

fn spawn_settings(mut commands: Commands) {
    commands.spawn((
        Name::new("Key settings"),
        DespawnOnExit(Screen::Settings),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(10.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.08, 0.12, 0.55)),
        children![
            text("TASTEN", 44.0),
            (
                BindingList,
                Node {
                    width: Val::Px(620.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
            ),
            (EditorHint, colored_text("", 16.0, palette::TEXT)),
            colored_text(
                "↑/↓/←/→ wählen   Enter ändern   Entf löschen   Esc zurück",
                15.0,
                palette::MUTED,
            ),
        ],
    ));
}

fn background(selected: bool) -> BackgroundColor {
    BackgroundColor(if selected {
        palette::HIGHLIGHT
    } else {
        palette::PANEL
    })
}

/// One key slot: its label, "Taste?" while waiting for a key, or "–" when empty.
fn slot_box(label: String, selected: bool, waiting: bool) -> impl Bundle {
    let (label, border) = if waiting {
        ("Taste?".to_string(), palette::CHECKPOINT)
    } else if selected {
        (label, palette::TEXT)
    } else {
        (label, Color::NONE)
    };
    (
        Node {
            width: Val::Px(150.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
            justify_content: JustifyContent::Center,
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(palette::PANEL),
        children![text(label, 18.0)],
    )
}

fn row_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        column_gap: Val::Px(10.0),
        border_radius: BorderRadius::all(Val::Px(8.0)),
        ..default()
    }
}

fn draw_settings(
    mut commands: Commands,
    editor: Res<BindingEditor>,
    bindings: Res<KeyBindings>,
    lists: Query<Entity, With<BindingList>>,
    mut hints: Query<&mut Text, With<EditorHint>>,
) {
    for list in &lists {
        commands.entity(list).despawn_related::<Children>();
        for (row, action) in Action::ALL.into_iter().enumerate() {
            let selected = editor.row == row;
            commands.entity(list).with_children(|list| {
                list.spawn((row_node(), background(selected)))
                    .with_children(|line| {
                        line.spawn(text(action.name(), 20.0));
                        line.spawn(Node {
                            column_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|boxes| {
                            for slot in 0..SLOTS {
                                let label = bindings
                                    .slot(action, slot)
                                    .map_or("–".to_string(), |key| key.label.clone());
                                let here = selected && editor.slot == slot;
                                boxes.spawn(slot_box(label, here, here && editor.waiting));
                            }
                        });
                    });
            });
        }
        commands.entity(list).with_child((
            row_node(),
            background(editor.row == RESET_ROW),
            children![text("Standardtasten wiederherstellen", 20.0)],
        ));
    }

    let hint = if let Some(notice) = &editor.notice {
        notice.clone()
    } else if let (true, Some(action)) = (editor.waiting, editor.action()) {
        format!(
            "Neue Taste für „{}“ drücken – Esc bricht ab.",
            action.name()
        )
    } else {
        let missing: Vec<&str> = Action::ALL
            .into_iter()
            .filter(|action| bindings.keys(*action).next().is_none())
            .map(Action::name)
            .collect();
        if missing.is_empty() {
            String::new()
        } else {
            format!("Ohne Taste: {}", missing.join(", "))
        }
    };
    for mut text in &mut hints {
        if text.0 != hint {
            text.0.clone_from(&hint);
        }
    }
}
