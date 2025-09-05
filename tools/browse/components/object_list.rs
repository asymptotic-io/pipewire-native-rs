// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use tui_realm_stdlib::List;
use tuirealm::{
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Alignment, Color, Style, TableBuilder},
    Component, Event, MockComponent, NoUserEvent, State, StateValue,
};

use crate::{ComponentId, Msg};

#[derive(MockComponent)]
pub struct ObjectList {
    component: List,
}

impl Default for ObjectList {
    fn default() -> Self {
        Self {
            component: List::default()
                .inactive(Style::default().fg(Color::Magenta))
                .scroll(true)
                .rewind(true)
                .title("Objects", Alignment::Left)
                .highlighted_str(" ⋄ ")
                .highlighted_color(Color::DarkGray)
                .rows(TableBuilder::default().build()),
        }
    }
}

impl Component<Msg, NoUserEvent> for ObjectList {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let old_selection = self.component.state();
        let mut focus_component = ComponentId::Objects;

        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Left, ..
            }) => {
                focus_component = ComponentId::Types;
                CmdResult::None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Right, ..
            }) => {
                focus_component = ComponentId::Details;
                CmdResult::None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                self.perform(Cmd::Move(Direction::Up))
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                ..
            }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Home, ..
            }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => {
                self.perform(Cmd::GoTo(Position::End))
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => return Some(Msg::Quit),
            Event::Keyboard(KeyEvent {
                code: Key::Char('p'),
                ..
            }) => return Some(Msg::ShowParams(true)),
            _ => CmdResult::None,
        };

        let new_selection = self.component.state();
        if old_selection != new_selection {
            if let State::One(StateValue::Usize(idx)) = new_selection {
                Some(Msg::ObjectChanged(idx))
            } else {
                Some(Msg::None)
            }
        } else if focus_component != ComponentId::Objects {
            Some(Msg::FocusChanged(focus_component))
        } else {
            Some(Msg::None)
        }
    }
}
