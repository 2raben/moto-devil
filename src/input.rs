//! Device input: keyboard and gamepad become [`Controls`] for the fixed-step
//! simulation and [`MenuAction`]s for the game flow. This is the only module that
//! reads devices.

use bevy::input::InputSystems;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::bike::Controls;
use crate::flow::{MenuAction, MenuInput, Screen};

/// Stick deflection below this is ignored.
const STICK_DEAD_ZONE: f32 = 0.25;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        // Sample before the fixed-step loop so this frame's presses reach this
        // frame's fixed steps.
        app.add_systems(
            PreUpdate,
            (
                sample_controls.run_if(in_state(Screen::Playing)),
                collect_menu_actions,
            )
                .after(InputSystems),
        );
    }
}

/// Keys are physical positions, so WASD and E stay in place on other layouts.
fn sample_controls(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut controls: ResMut<Controls>,
) {
    let held = |codes: &[KeyCode]| f32::from(u8::from(keys.any_pressed(codes.iter().copied())));
    let pressed = |codes: &[KeyCode]| keys.any_just_pressed(codes.iter().copied());
    const GAS: &[KeyCode] = &[KeyCode::KeyW, KeyCode::ArrowUp];
    const BRAKE: &[KeyCode] = &[KeyCode::KeyS, KeyCode::ArrowDown];
    const LEFT: &[KeyCode] = &[KeyCode::KeyA, KeyCode::ArrowLeft];
    const RIGHT: &[KeyCode] = &[KeyCode::KeyD, KeyCode::ArrowRight];

    let mut throttle = held(GAS);
    let mut brake = held(BRAKE);
    let mut tilt = held(RIGHT) - held(LEFT);
    let mut jump = pressed(&[KeyCode::Space]);
    let mut turn = pressed(&[KeyCode::KeyE]);
    let mut restart = pressed(&[KeyCode::KeyR]);
    for pad in &gamepads {
        throttle = throttle.max(pad.get(GamepadButton::RightTrigger2).unwrap_or(0.0));
        brake = brake.max(pad.get(GamepadButton::LeftTrigger2).unwrap_or(0.0));
        let stick = pad.left_stick().x;
        if stick.abs() > STICK_DEAD_ZONE {
            tilt += stick;
        }
        tilt += pad.dpad().x;
        jump |= pad.just_pressed(GamepadButton::South);
        turn |= pad.just_pressed(GamepadButton::West);
        restart |= pad.just_pressed(GamepadButton::Select);
    }

    controls.throttle = throttle.clamp(0.0, 1.0);
    controls.brake = brake.clamp(0.0, 1.0);
    controls.tilt = tilt.clamp(-1.0, 1.0);
    // Latch presses until a fixed step consumes them.
    controls.jump_pressed |= jump;
    controls.turn_pressed |= turn;
    controls.restart |= restart;
}

/// Menu keys and typed text. On the player selection every letter and the
/// space bar type, so they are not read as riding keys there.
fn collect_menu_actions(
    screen: Res<State<Screen>>,
    mut keyboard: MessageReader<KeyboardInput>,
    gamepads: Query<&Gamepad>,
    mut menu: ResMut<MenuInput>,
) {
    let selecting = *screen.get() == Screen::SelectPlayer;
    for event in keyboard.read() {
        if !event.state.is_pressed() {
            continue;
        }
        let action = match event.key_code {
            KeyCode::Enter | KeyCode::NumpadEnter if !event.repeat => Some(MenuAction::Confirm),
            KeyCode::Escape if !event.repeat => Some(MenuAction::Back),
            KeyCode::ArrowUp if selecting => Some(MenuAction::Up),
            KeyCode::ArrowDown if selecting => Some(MenuAction::Down),
            KeyCode::Backspace if selecting => Some(MenuAction::Erase),
            _ => None,
        };
        if let Some(action) = action {
            menu.0.push(action);
        } else if selecting && let Some(text) = &event.text {
            menu.0.extend(
                text.chars()
                    .filter(|c| !c.is_control())
                    .map(MenuAction::Type),
            );
        }
    }
    for pad in &gamepads {
        if pad.just_pressed(GamepadButton::South) {
            menu.0.push(MenuAction::Confirm);
        }
        if selecting && pad.just_pressed(GamepadButton::DPadUp) {
            menu.0.push(MenuAction::Up);
        }
        if selecting && pad.just_pressed(GamepadButton::DPadDown) {
            menu.0.push(MenuAction::Down);
        }
        if !selecting && pad.just_pressed(GamepadButton::Start) {
            menu.0.push(MenuAction::Back);
        }
    }
}
