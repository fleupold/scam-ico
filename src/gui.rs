use std::collections::HashMap;
use std::io::{self, Result as IoResult, Stdout};
use std::process;
use termion::event::Key;
use termion::screen::AlternateScreen;
use termion::input::{MouseTerminal, TermRead};
use termion::raw::{IntoRawMode, RawTerminal};
use tui::backend::TermionBackend;
use tui::{Frame, Terminal};

type Backend = TermionBackend<AlternateScreen<MouseTerminal<RawTerminal<Stdout>>>>;

pub struct Gui<'a> {
    terminal: Terminal<Backend>,
    actions: HashMap<Key, Box<dyn FnMut() -> Control + 'a>>,
}

impl<'a> Gui<'a> {
    pub fn new() -> IoResult<Gui<'a>> {
        let stdout = io::stdout().into_raw_mode()?;
        let stdout = MouseTerminal::from(stdout);
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);

        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;

        Ok(Gui {
            terminal,
            actions: HashMap::new(),
        })
    }

    pub fn with_action<F>(mut self, key: Key, action: F) -> Self
    where
        F: FnMut() -> Control + 'a,
    {
        self.actions.insert(key, Box::new(action));
        self
    }

    pub fn run<F>(self, mut draw: F) -> IoResult<i32>
    where
        F: FnMut(Frame<'_, Backend>),
    {
        let Gui {
            mut terminal,
            mut actions,
        } = self;

        terminal.draw(|f| draw(f))?;
        for key in io::stdin().keys() {
            let key = key?;
            if let Some(action) = actions.get_mut(&key) {
                match action() {
                    Control::Continue => {
                        terminal.draw(|f| draw(f))?;
                        continue;
                    }
                    Control::Quit(code) => return Ok(code),
                }
            }
        }

        unreachable!();
    }
}

pub enum Control {
    Continue,
    Quit(i32),
}
