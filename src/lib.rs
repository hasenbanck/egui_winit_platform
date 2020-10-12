//! A platform integration to use [egui](https://github.com/emilk/egui) with [winit](https://github.com/rust-windowing/winit).
//!
//! You need to create a [`Platform`] and feed it with `winit::event::Event` events.
//! Use `begin_frame()` and `end_frame()` to start drawing the egui UI.
//! A basic usage example can be found [here](https://github.com/hasenbanck/egui_example).
#![warn(missing_docs)]

use std::sync::Arc;

use egui::Key;
use egui::math::{pos2, vec2};
use winit::event::{Event, ModifiersState, VirtualKeyCode};
use winit::event::VirtualKeyCode::*;
use winit::event::WindowEvent::*;

/// Configures the creation of the `Platform`.
pub struct PlatformDescriptor {
    /// Width of the window in physical pixel.
    pub physical_width: u32,
    /// Height of the window in physical pixel.
    pub physical_height: u32,
    /// HiDPI scale factor.
    pub scale_factor: f64,
    /// Egui font configuration.
    pub font_definitions: egui::paint::fonts::FontDefinitions,
    /// Egui style configuration.
    pub style: egui::Style,
}

/// Provides the integration between egui and winit.
pub struct Platform {
    scale_factor: f64,
    context: Arc<egui::Context>,
    raw_input: egui::RawInput,
    modifier_state: ModifiersState,
}

impl Platform {
    /// Creates a new `Platform`.
    pub fn new(descriptor: PlatformDescriptor) -> Self {
        let context = egui::Context::new();

        context.set_fonts(descriptor.font_definitions.clone());
        context.set_style(descriptor.style);

        let mut raw_input = egui::RawInput::default();
        raw_input.pixels_per_point = Some(descriptor.font_definitions.pixels_per_point);

        raw_input.screen_size =
            vec2(descriptor.physical_width as f32, descriptor.physical_height as f32)
                / descriptor.scale_factor as f32;

        Self {
            scale_factor: descriptor.scale_factor,
            context,
            raw_input,
            modifier_state: winit::event::ModifiersState::empty(),
        }
    }

    /// Handles the given winit event and updates the egui context. Should be called before starting a new frame with `start_frame()`.
    pub fn handle_event<T>(&mut self, winit_event: &Event<T>) {
        match winit_event {
            Event::WindowEvent {
                window_id: _window_id,
                event,
            } => match event {
                Resized(physical_size) => {
                    self.raw_input.screen_size =
                        vec2(physical_size.width as f32, physical_size.height as f32)
                            / self.scale_factor as f32;
                }
                ScaleFactorChanged {
                    scale_factor,
                    new_inner_size,
                } => {
                    self.scale_factor = *scale_factor;
                    self.raw_input.pixels_per_point = Some(*scale_factor as f32);
                    self.raw_input.screen_size =
                        vec2(new_inner_size.width as f32, new_inner_size.height as f32)
                            / self.scale_factor as f32;
                }
                MouseInput { state, .. } => {
                    self.raw_input.mouse_down = *state == winit::event::ElementState::Pressed;
                }
                MouseWheel { delta, .. } => {
                    match delta {
                        winit::event::MouseScrollDelta::LineDelta(x, y) => {
                            let line_height = 24.0; // TODO as in egui_glium
                            self.raw_input.scroll_delta = vec2(*x, *y) * line_height;
                        }
                        winit::event::MouseScrollDelta::PixelDelta(delta) => {
                            // Actually point delta
                            self.raw_input.scroll_delta = vec2(delta.x as f32, delta.y as f32);
                        }
                    }
                }
                CursorMoved { position, .. } => {
                    self.raw_input.mouse_pos = Some(pos2(
                        position.x as f32 / self.raw_input.pixels_per_point.unwrap(),
                        position.y as f32 / self.raw_input.pixels_per_point.unwrap(),
                    ));
                }
                CursorLeft { .. } => {
                    self.raw_input.mouse_pos = None;
                }
                ModifiersChanged(input) => self.modifier_state = *input,
                KeyboardInput { input, .. } => {
                    if let Some(virtual_keycode) = input.virtual_keycode {
                        match virtual_keycode {
                            VirtualKeyCode::Copy => self.raw_input.events.push(egui::Event::Copy),
                            VirtualKeyCode::Cut => self.raw_input.events.push(egui::Event::Cut),
                            _ => {
                                if let Some(key) = winit_to_egui_key_code(virtual_keycode) {
                                    self.raw_input.events.push(egui::Event::Key {
                                        key,
                                        pressed: input.state == winit::event::ElementState::Pressed,
                                    });
                                }
                            }
                        }
                    }
                }
                ReceivedCharacter(ch) => {
                    if is_printable(*ch) {
                        self.raw_input
                            .events
                            .push(egui::Event::Text(ch.to_string()));
                    }
                }
                _ => {}
            },
            Event::DeviceEvent { .. } => {}
            _ => {}
        }
    }

    /// Updates the internal time for egui used for animations. `elapsed_seconds` should be the seconds since some point in time (for example application start).
    pub fn update_time(&mut self, elapsed_seconds: f64) {
        self.raw_input.time = elapsed_seconds;
    }

    /// Starts a new frame by providing a new `Ui` instance to write into.
    pub fn begin_frame(&mut self) -> egui::Ui {
        self.context.begin_frame(self.raw_input.take())
    }

    /// Ends the frame. Returns what has happened as `Output` and gives you the draw instructions as `PaintJobs`.
    pub fn end_frame(&self) -> (egui::Output, egui::PaintJobs) {
        self.context.end_frame()
    }

    /// Returns the internal egui context.
    pub fn context(&self) -> Arc<egui::Context> {
        self.context.clone()
    }
}

/// Translates winit to egui keycodes.
#[inline]
fn winit_to_egui_key_code(key: VirtualKeyCode) -> Option<egui::Key> {
    Some(match key {
        Escape => Key::Escape,
        Insert => Key::Insert,
        Home => Key::Home,
        Delete => Key::Delete,
        End => Key::End,
        PageDown => Key::PageDown,
        PageUp => Key::PageUp,
        Left => Key::Left,
        Up => Key::Up,
        Right => Key::Right,
        Down => Key::Down,
        Back => Key::Backspace,
        Return => Key::Enter,
        Tab => Key::Tab,
        LAlt | RAlt => Key::Alt,
        LShift | RShift => Key::Shift,
        LControl | RControl => Key::Control,
        LWin | RWin => Key::Logo,
        _ => {
            return None;
        }
    })
}

/// We only want printable characters and ignore all special keys.
#[inline]
fn is_printable(chr: char) -> bool {
    let is_in_private_use_area = '\u{e000}' <= chr && chr <= '\u{f8ff}'
        || '\u{f0000}' <= chr && chr <= '\u{ffffd}'
        || '\u{100000}' <= chr && chr <= '\u{10fffd}';

    !is_in_private_use_area && !chr.is_ascii_control()
}
