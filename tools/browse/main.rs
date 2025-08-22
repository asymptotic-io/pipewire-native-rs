// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use pipewire::{keys, proxy::HasProxy};
use tui_realm_stdlib::List;
use tuirealm::{
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Alignment, Color, Style, TableBuilder, TextSpan},
    ratatui::layout,
    terminal::{CrosstermTerminalAdapter, TerminalBridge},
    Application, AttrValue, Attribute, Component, Event, EventListenerCfg, MockComponent,
    NoUserEvent, PollStrategy, State, StateValue, Update,
};

mod pw;

#[derive(Debug, Eq, PartialEq, Clone)]
enum Msg {
    FocusChanged(ComponentId),
    TypeChanged(TypeSelection),
    ObjectChanged(usize),
    Quit,
    None,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
enum ComponentId {
    Types,
    Objects,
    Details,
}

struct Model {
    app: Application<ComponentId, Msg, NoUserEvent>,
    pw_state: Arc<pw::State>,
    component_selection: ComponentId,
    type_selection: TypeSelection,
    object_selection: usize,
    quit: bool,
    redraw: bool,
}

impl Model {
    fn new(pw_state: Arc<pw::State>) -> Self {
        let mut app = Application::init(
            EventListenerCfg::default().crossterm_input_listener(Duration::from_millis(20), 10),
        );

        app.mount(ComponentId::Types, Box::new(TypesList::default()), vec![])
            .unwrap();
        app.mount(
            ComponentId::Objects,
            Box::new(ObjectsList::default()),
            vec![],
        )
        .unwrap();

        app.mount(
            ComponentId::Details,
            Box::new(ObjectDetails::default()),
            vec![],
        )
        .unwrap();

        app.active(&ComponentId::Types).unwrap();

        Self {
            app,
            pw_state,
            component_selection: ComponentId::Types,
            type_selection: TypeSelection::Clients,
            object_selection: 0,
            quit: false,
            redraw: true,
        }
    }

    fn view(&mut self, terminal: &mut TerminalBridge<CrosstermTerminalAdapter>) {
        let _ = terminal.raw_mut().draw(|f| {
            let layout = layout::Layout::default()
                .direction(layout::Direction::Horizontal)
                .constraints([
                    layout::Constraint::Percentage(10),
                    layout::Constraint::Percentage(30),
                    layout::Constraint::Percentage(60),
                ])
                .split(f.area());

            self.app.view(&ComponentId::Types, f, layout[0]);
            self.app.view(&ComponentId::Objects, f, layout[1]);
            self.app.view(&ComponentId::Details, f, layout[2]);
        });
    }

    fn update_object_list(&mut self) {
        let mut table = TableBuilder::default();
        let guard = self.pw_state.main_loop.lock();

        match self.type_selection {
            TypeSelection::Clients => {
                let clients = self.pw_state.clients.borrow();
                let n = clients.len();

                for (idx, (id, (client, props))) in clients.iter().enumerate() {
                    table.add_col(TextSpan::from(format!(
                        "#{}: {}",
                        client.proxy().bound_id().unwrap_or(*id),
                        props.get(keys::APP_NAME).unwrap_or("unknown"),
                    )));

                    // Only add rows between columns
                    if idx < n - 1 {
                        table.add_row();
                    }
                }

                self.app
                    .attr(
                        &ComponentId::Objects,
                        Attribute::Content,
                        AttrValue::Table(table.build()),
                    )
                    .unwrap();
            }
            TypeSelection::Modules => {
                let modules = self.pw_state.modules.borrow();
                let n = modules.len();

                for (idx, (id, (client, props))) in modules.iter().enumerate() {
                    table.add_col(TextSpan::from(format!(
                        "#{}: {}",
                        client.proxy().bound_id().unwrap_or(*id),
                        props.get("module.name").unwrap_or("unknown"),
                    )));

                    // Only add rows between columns
                    if idx < n - 1 {
                        table.add_row();
                    }
                }

                self.app
                    .attr(
                        &ComponentId::Objects,
                        Attribute::Content,
                        AttrValue::Table(table.build()),
                    )
                    .unwrap();
            }
        }

        drop(guard);

        self.update_object_details();
    }

    fn update_object_details(&mut self) {
        let props = match self.type_selection {
            TypeSelection::Clients => {
                let _guard = self.pw_state.main_loop.lock();

                let clients = self.pw_state.clients.borrow();
                let entries = clients.iter().collect::<Vec<_>>();

                if let Some(entry) = entries.get(self.object_selection) {
                    let mut props = entry.1 .1.iter().collect::<Vec<(&str, &str)>>();
                    props.sort_by_key(|e| e.0);

                    props
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect()
                } else {
                    vec![]
                }
            }
            TypeSelection::Modules => {
                let _guard = self.pw_state.main_loop.lock();

                let modules = self.pw_state.modules.borrow();
                let entries = modules.iter().collect::<Vec<_>>();

                if let Some(entry) = entries.get(self.object_selection) {
                    let mut props = entry.1 .1.iter().collect::<Vec<_>>();
                    props.sort_by_key(|e| e.0);

                    props
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect()
                } else {
                    vec![]
                }
            }
        };

        let n = props.len();

        let mut table = TableBuilder::default();

        for (idx, (key, value)) in props.iter().enumerate() {
            table.add_col(TextSpan::from(key).fg(Color::Cyan));
            table.add_col(TextSpan::from(" "));
            table.add_col(TextSpan::from(value));

            // Only add rows between columns
            if idx < n - 1 {
                table.add_row();
            }
        }

        self.app
            .attr(
                &ComponentId::Details,
                Attribute::Content,
                AttrValue::Table(table.build()),
            )
            .unwrap();
    }
}

impl Update<Msg> for Model {
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        self.redraw = true;

        match msg.unwrap_or(Msg::None) {
            Msg::FocusChanged(component_id) => {
                self.component_selection = component_id;
                self.app.active(&self.component_selection).unwrap();
                None
            }
            Msg::TypeChanged(type_selection) => {
                self.type_selection = type_selection;
                self.update_object_list();
                None
            }
            Msg::ObjectChanged(idx) => {
                self.object_selection = idx;
                self.update_object_details();
                None
            }
            Msg::Quit => {
                self.quit = true;
                None
            }
            Msg::None => None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
enum TypeSelection {
    Clients,
    Modules,
}

impl TryFrom<usize> for TypeSelection {
    type Error = ();
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TypeSelection::Clients),
            1 => Ok(TypeSelection::Modules),
            _ => Err(()),
        }
    }
}

#[derive(MockComponent)]
struct TypesList {
    component: List,
}

impl Default for TypesList {
    fn default() -> Self {
        Self {
            component: List::default()
                .inactive(Style::default().fg(Color::Magenta))
                .scroll(true)
                .rewind(true)
                .title("Object types", Alignment::Left)
                .highlighted_str(" ⋄ ")
                .highlighted_color(Color::DarkGray)
                .rows(
                    TableBuilder::default()
                        .add_col(TextSpan::from("Clients"))
                        .add_row()
                        .add_col(TextSpan::from("Modules"))
                        .build(),
                ),
        }
    }
}

impl Component<Msg, NoUserEvent> for TypesList {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let old_selection = self.component.state();
        let mut focus_changed = false;

        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Right, ..
            }) => {
                focus_changed = true;
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
            _ => CmdResult::None,
        };

        let new_selection = self.component.state();
        if old_selection != new_selection {
            if let State::One(StateValue::Usize(idx)) = new_selection {
                Some(Msg::TypeChanged(TypeSelection::try_from(idx).unwrap()))
            } else {
                Some(Msg::None)
            }
        } else if focus_changed {
            Some(Msg::FocusChanged(ComponentId::Objects))
        } else {
            Some(Msg::None)
        }
    }
}

#[derive(MockComponent)]
struct ObjectsList {
    component: List,
}

impl Default for ObjectsList {
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

impl Component<Msg, NoUserEvent> for ObjectsList {
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

#[derive(MockComponent)]
struct ObjectDetails {
    component: List,
}

impl Default for ObjectDetails {
    fn default() -> Self {
        Self {
            component: List::default()
                .inactive(Style::default().fg(Color::Magenta))
                .scroll(true)
                .rewind(true)
                .title("Details", Alignment::Left)
                .highlighted_str(" ")
                .rows(TableBuilder::default().build()),
        }
    }
}

impl Component<Msg, NoUserEvent> for ObjectDetails {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let mut focus_changed = false;

        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Left, ..
            }) => {
                focus_changed = true;
                CmdResult::None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                self.perform(Cmd::Scroll(Direction::Up))
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
            _ => CmdResult::None,
        };

        if focus_changed {
            Some(Msg::FocusChanged(ComponentId::Objects))
        } else {
            Some(Msg::None)
        }
    }
}

fn main() {
    let mut terminal =
        TerminalBridge::init_crossterm().expect("Could not initialise terminal bridge");
    let _ = terminal.enable_raw_mode();
    let _ = terminal.enter_alternate_screen();

    let pw_update = Arc::new(AtomicBool::new(false));
    let pw_state = pw::State::new("pw-browse", pw_update.clone())
        .expect("PipeWire initialisation should succeed");
    let mut model = Model::new(pw_state);

    model.pw_state.run();

    // And use this main thread as the UI event loop
    while !model.quit {
        if let Ok(messages) = model.app.tick(PollStrategy::Once) {
            for msg in messages {
                let mut msg = Some(msg);
                while msg.is_some() {
                    msg = model.update(msg);
                }
            }

            // We have an update from PipeWire
            if pw_update.swap(false, Ordering::Relaxed) {
                model.update_object_list();
                model.redraw = true;
            }

            if model.redraw {
                model.view(&mut terminal);
                model.redraw = false;
            }
        }
    }

    model.pw_state.stop();

    let _ = terminal.leave_alternate_screen();
    let _ = terminal.disable_raw_mode();
}
