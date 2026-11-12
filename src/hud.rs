//! On-screen text while riding: progress, a controls reminder, short cues, and
//! the finish panel. Also replaces Bevy's built-in font, which has no umlauts,
//! with DejaVu Sans Bold (`assets/fonts/`, see its license file there).

use bevy::prelude::*;

use crate::effects::{JET_SECONDS, RideEffects};
use crate::flow::{CurrentLevel, Levels, Screen};
use crate::level::Level;
use crate::progress::{ActivePlayer, Profiles};
use crate::run::Run;
use crate::visuals::palette;

/// The country and level are announced this long at the start of a level.
const INTRO_SECONDS: f32 = 2.0;

const FONT: &[u8] = include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf");

#[derive(Component)]
struct Progress;

#[derive(Component)]
struct Cue;

#[derive(Component)]
struct FinishPanel;

#[derive(Component)]
struct EffectPanel;
#[derive(Component)]
struct EffectText;
#[derive(Component)]
struct JetBar;
#[derive(Component)]
struct JetGauge;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        // Text without an explicit font uses the default handle; give it ours.
        if let Some(mut fonts) = app.world_mut().get_resource_mut::<Assets<Font>>()
            && fonts
                .insert(AssetId::default(), Font::from_bytes(FONT.to_vec()))
                .is_err()
        {
            warn!("Could not replace the default font");
        }
        app.add_systems(OnEnter(Screen::Playing), spawn_hud)
            .add_systems(
                Update,
                (show_run, show_effects).run_if(in_state(Screen::Playing)),
            );
    }
}

pub fn text(content: impl Into<String>, size: f32) -> impl Bundle {
    colored_text(content, size, palette::TEXT)
}

pub fn colored_text(content: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(content),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn panel(node: Node) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
            ..node
        },
        BackgroundColor(palette::PANEL),
    )
}

fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Bike effects"),
            EffectPanel,
            DespawnOnExit(Screen::Playing),
            panel(Node {
                top: Val::Px(54.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            }),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((EffectText, text("", 16.0)));
            parent
                .spawn((
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(8.0),
                        ..default()
                    },
                    BackgroundColor(palette::ROCK),
                    JetGauge,
                ))
                .with_child((
                    JetBar,
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(palette::HIGHLIGHT),
                ));
        });
    commands.spawn((
        Name::new("Progress"),
        Progress,
        DespawnOnExit(Screen::Playing),
        panel(Node {
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
        text("", 20.0),
    ));
    commands.spawn((
        Name::new("Controls reminder"),
        DespawnOnExit(Screen::Playing),
        panel(Node {
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
        text(
            "W/↑ Gas   S/↓ Bremse   A/← D/→ kippen   Leertaste springen   E umdrehen   R neu   Esc Spielerauswahl",
            14.0,
        ),
    ));
    commands.spawn((
        Name::new("Cue"),
        Cue,
        DespawnOnExit(Screen::Playing),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(18.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        text("", 40.0),
        TextLayout::justify(Justify::Center),
        TextShadow::default(),
        Visibility::Hidden,
    ));
    commands.spawn((
        Name::new("Finish panel"),
        FinishPanel,
        DespawnOnExit(Screen::Playing),
        panel(Node {
            top: Val::Percent(32.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        }),
        text("", 30.0),
        TextLayout::justify(Justify::Center),
        Visibility::Hidden,
    ));
}

fn show_effects(
    bikes: Query<&RideEffects>,
    mut panels: Query<&mut Visibility, With<EffectPanel>>,
    mut labels: Query<&mut Text, With<EffectText>>,
    mut bars: Query<&mut Node, (With<JetBar>, Without<JetGauge>)>,
    mut gauges: Query<&mut Node, (With<JetGauge>, Without<JetBar>)>,
) {
    let ride = bikes.iter().next();
    let mut messages = Vec::new();
    if let Some(ride) = ride {
        if ride.stuck.is_some() {
            messages.push("Kaktus! Springen befreit dich.".to_string());
        }
        if ride.muddy {
            messages.push("Matsch: neuer Anlauf nötig".to_string());
        }
        if ride.sand_surface.is_some() {
            messages.push("Treibsand! Nicht anhalten – springen!".to_string());
        }
        if ride.jet_left > 0.0 {
            messages.push(format!(
                "Jet {} s · Leertaste / Süd halten zum Fliegen",
                seconds(ride.jet_left)
            ));
        }
    }
    for mut v in &mut panels {
        *v = if messages.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for mut text in &mut labels {
        text.0 = messages.join("\n");
    }
    for mut node in &mut gauges {
        node.display = if ride.is_some_and(|r| r.jet_left > 0.0) {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut node in &mut bars {
        node.width = Val::Percent(100.0 * ride.map_or(0.0, |r| r.jet_left / JET_SECONDS));
    }
}

/// Seconds with one decimal and a decimal comma, as written in German.
fn seconds(value: f32) -> String {
    format!("{value:.1}").replace('.', ",")
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn show_run(
    run: Res<Run>,
    level: Option<Res<Level>>,
    levels: Res<Levels>,
    current: Res<CurrentLevel>,
    profiles: Res<Profiles>,
    active: Res<ActivePlayer>,
    mut progress: Query<&mut Text, (With<Progress>, Without<Cue>, Without<FinishPanel>)>,
    mut cue: Query<(&mut Text, &mut Visibility), (With<Cue>, Without<FinishPanel>)>,
    mut finish: Query<(&mut Text, &mut Visibility), (With<FinishPanel>, Without<Cue>)>,
) {
    let name = active
        .0
        .and_then(|player| profiles.players.get(player))
        .map_or("", |player| player.name.as_str());
    let level_name = level.as_ref().map_or("", |level| level.name);
    let place = levels.place(current.0);
    let where_text = place.map_or(String::new(), |(country, number, count)| {
        format!("{} {number}/{count} · ", country.name())
    });
    let stats = format!(
        "Zeit {} s   Tode {}   Flips {}",
        seconds(run.elapsed),
        run.deaths,
        run.flips
    );
    for mut text in &mut progress {
        let summary = format!("{name} · {where_text}{level_name}   {stats}");
        if text.0 != summary {
            text.0 = summary;
        }
    }

    let message = if run.crash.is_some() {
        Some("Autsch!".to_string())
    } else if let Some((flips, _)) = run.flip_cue {
        Some(if flips == 1 {
            "Flip!".to_string()
        } else {
            format!("{flips}× Flip!")
        })
    } else if run.checkpoint_cue.is_some() {
        Some("Checkpoint!".to_string())
    } else if run.elapsed < INTRO_SECONDS && run.deaths == 0 && run.attempt == 0 {
        place.map(|(country, number, count)| {
            format!("{}\n{number}/{count} · {level_name}", country.name())
        })
    } else {
        None
    };
    for (mut text, mut visibility) in &mut cue {
        match &message {
            Some(message) if !run.finished => {
                if &text.0 != message {
                    text.0.clone_from(message);
                }
                visibility.set_if_neq(Visibility::Inherited);
            }
            _ => {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
    }

    for (mut text, mut visibility) in &mut finish {
        if run.finished {
            let (title, next) = if let Some(country) = levels.next_country(current.0) {
                let done = place.map_or("", |(done, ..)| done.name());
                (
                    format!("{done} geschafft!"),
                    format!("Enter: weiter nach {}", country.name()),
                )
            } else if current.0 + 1 < levels.0.len() {
                ("Ziel!".to_string(), "Enter: nächstes Level".to_string())
            } else {
                (
                    "Alle Länder geschafft!".to_string(),
                    "Enter: zur Spielerauswahl".to_string(),
                )
            };
            let summary = format!("{title}\n{stats}\n{next}   R: nochmal");
            if text.0 != summary {
                text.0 = summary;
            }
            visibility.set_if_neq(Visibility::Inherited);
        } else {
            visibility.set_if_neq(Visibility::Hidden);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_use_a_decimal_comma() {
        assert_eq!(seconds(12.34), "12,3");
    }
}
