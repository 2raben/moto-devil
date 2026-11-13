//! Device input: keyboard and gamepad become [`Controls`] for the fixed-step
//! simulation and [`MenuAction`]s for the game flow. Riding keys come from the
//! player's [`KeyBindings`]. This module and [`crate::bindings`] are the only
//! ones that read devices.

use bevy::input::InputSystems;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::bike::Controls;
use crate::bindings::{self, Action, BindingEditor, KeyBindings};
use crate::flow::{MenuAction, MenuInput, Screen};

/// Stick deflection below this is ignored.
const STICK_DEAD_ZONE: f32 = 0.25;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        // Sample before the fixed-step loop so this frame's presses reach this
        // frame's fixed steps.
        app.init_resource::<KeyBindings>()
            .init_resource::<BindingEditor>()
            .add_systems(
                PreUpdate,
                (
                    sample_controls.run_if(in_state(Screen::Playing)),
                    collect_menu_actions,
                    bindings::edit_bindings,
                )
                    .after(InputSystems),
            )
            .add_systems(OnEnter(Screen::Settings), bindings::reset_editor);
    }
}

/// Keys are physical positions, so WASD and E stay in place on other layouts.
fn sample_controls(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<KeyBindings>,
    gamepads: Query<&Gamepad>,
    mut controls: ResMut<Controls>,
) {
    let down = |action| keys.any_pressed(bindings.keys(action));
    let held = |action| f32::from(u8::from(down(action)));
    let pressed = |action| keys.any_just_pressed(bindings.keys(action));

    let mut throttle = held(Action::Gas);
    let mut brake = held(Action::Brake);
    let mut tilt = held(Action::TiltRight) - held(Action::TiltLeft);
    let mut jump_held = down(Action::Jump);
    let mut jump = pressed(Action::Jump);
    let mut turn = pressed(Action::Turn);
    let mut restart = pressed(Action::Restart);
    for pad in &gamepads {
        throttle = throttle.max(pad.get(GamepadButton::RightTrigger2).unwrap_or(0.0));
        brake = brake.max(pad.get(GamepadButton::LeftTrigger2).unwrap_or(0.0));
        let stick = pad.left_stick().x;
        if stick.abs() > STICK_DEAD_ZONE {
            tilt += stick;
        }
        tilt += pad.dpad().x;
        jump_held |= pad.pressed(GamepadButton::South);
        jump |= pad.just_pressed(GamepadButton::South);
        turn |= pad.just_pressed(GamepadButton::West);
        restart |= pad.just_pressed(GamepadButton::Select);
    }

    controls.throttle = throttle.clamp(0.0, 1.0);
    controls.brake = brake.clamp(0.0, 1.0);
    controls.tilt = tilt.clamp(-1.0, 1.0);
    // Latch presses until a fixed step consumes them.
    controls.jump_pressed |= jump;
    controls.jump_held = jump_held;
    controls.turn_pressed |= turn;
    controls.restart |= restart;
}

/// Menu keys and typed text. On the player selection every letter and the
/// space bar type, so they are not read as riding keys there. On the key
/// settings [`bindings::edit_bindings`] reads the keyboard instead, and a
/// gamepad can only go back.
fn collect_menu_actions(
    screen: Res<State<Screen>>,
    mut keyboard: MessageReader<KeyboardInput>,
    gamepads: Query<&Gamepad>,
    mut menu: ResMut<MenuInput>,
) {
    if *screen.get() == Screen::Settings {
        keyboard.clear();
        for pad in &gamepads {
            if pad.any_just_pressed([GamepadButton::East, GamepadButton::Start]) {
                menu.0.push(MenuAction::Back);
            }
        }
        return;
    }
    let selecting = *screen.get() == Screen::SelectPlayer;
    for event in keyboard.read() {
        if !event.state.is_pressed() {
            continue;
        }
        let action = match event.key_code {
            KeyCode::Enter | KeyCode::NumpadEnter if !event.repeat => Some(MenuAction::Confirm),
            KeyCode::Escape if !event.repeat => Some(MenuAction::Back),
            KeyCode::Tab if selecting && !event.repeat => Some(MenuAction::Settings),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releasing_jump_stops_held_thrust_without_losing_the_latched_press() {
        let mut app = App::new();
        app.init_resource::<Controls>()
            .init_resource::<KeyBindings>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, sample_controls);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();
        assert!(app.world().resource::<Controls>().jump_held);
        assert!(app.world().resource::<Controls>().jump_pressed);
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.clear_just_pressed(KeyCode::Space);
            keys.release(KeyCode::Space);
        }
        app.update();
        assert!(!app.world().resource::<Controls>().jump_held);
        assert!(app.world().resource::<Controls>().jump_pressed);
        app.world_mut().resource_mut::<Controls>().jump_pressed = false;
        app.update();
        assert!(!app.world().resource::<Controls>().jump_pressed);
    }

    #[test]
    fn rebound_keys_ride_and_the_old_ones_do_not() {
        use crate::bindings::BoundKey;

        let mut bindings = KeyBindings::default();
        bindings.bind(Action::Gas, 0, BoundKey::new(KeyCode::KeyI, None).unwrap());
        bindings.bind(Action::Jump, 0, BoundKey::new(KeyCode::KeyW, None).unwrap());
        let mut app = App::new();
        app.init_resource::<Controls>()
            .insert_resource(bindings)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, sample_controls);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.update();
        let controls = app.world().resource::<Controls>();
        assert_eq!(controls.throttle, 0.0, "W now jumps");
        assert!(controls.jump_pressed && controls.jump_held);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyI);
        app.update();
        assert_eq!(app.world().resource::<Controls>().throttle, 1.0);
    }
}
