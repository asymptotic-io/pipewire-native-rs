// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

mod components;
mod pw;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use pipewire::properties::Properties;
use tuirealm::{
    props::{Color, TableBuilder, TextSpan},
    ratatui::layout,
    terminal::{CrosstermTerminalAdapter, TerminalBridge},
    Application, AttrValue, Attribute, EventListenerCfg, NoUserEvent, PollStrategy, Update,
};

use components::{
    object_details::ObjectDetails, object_list::ObjectList, type_list::TypeList,
    type_list::TypeSelection,
};

use crate::components::renderable::Renderable;

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

        app.mount(ComponentId::Types, Box::new(TypeList::default()), vec![])
            .unwrap();
        app.mount(
            ComponentId::Objects,
            Box::new(ObjectList::default()),
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
        let mut objects: Vec<(Box<dyn Renderable>, Properties)> = vec![];

        match self.type_selection {
            TypeSelection::Clients => {
                let clients = self.pw_state.clients.read().unwrap();

                for (_, (client, props)) in clients.iter() {
                    objects.push((Box::new(client.clone()), props.clone()));
                }
            }
            TypeSelection::Devices => {
                let devices = self.pw_state.devices.read().unwrap();

                for (_, (device, props)) in devices.iter() {
                    objects.push((Box::new(device.clone()), props.clone()));
                }
            }
            TypeSelection::Modules => {
                let modules = self.pw_state.modules.read().unwrap();

                for (_, (module, props)) in modules.iter() {
                    objects.push((Box::new(module.clone()), props.clone()))
                }
            }
        }

        let mut table = TableBuilder::default();

        for (idx, (object, props)) in objects.iter().enumerate() {
            table.add_col(object.title(props));

            if idx < objects.len() - 1 {
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
        self.update_object_details();
    }

    fn update_object_details(&mut self) {
        let props = match self.type_selection {
            TypeSelection::Clients => {
                let clients = self.pw_state.clients.read().unwrap();
                let entries = clients.iter().collect::<Vec<_>>();
                entries.get(self.object_selection).map(|e| e.1 .1.clone())
            }
            TypeSelection::Devices => {
                let devices = self.pw_state.devices.read().unwrap();
                let entries = devices.iter().collect::<Vec<_>>();
                entries.get(self.object_selection).map(|e| e.1 .1.clone())
            }
            TypeSelection::Modules => {
                let modules = self.pw_state.modules.read().unwrap();
                let entries = modules.iter().collect::<Vec<_>>();
                entries.get(self.object_selection).map(|e| e.1 .1.clone())
            }
        };

        if let Some(props) = props {
            let mut props_str = props.iter().collect::<Vec<_>>();
            props_str.sort_by_key(|e| e.0);

            let n = props_str.len();

            let mut table = TableBuilder::default();

            for (idx, (key, value)) in props_str.iter().enumerate() {
                table.add_col(TextSpan::from(*key).fg(Color::Cyan));
                table.add_col(TextSpan::from(" "));
                table.add_col(TextSpan::from(*value));

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
