use std::cmp;
use std::collections::HashMap;
use std::io::{self, Result as IoResult, Stdout};
use termion::event::Key;
use termion::input::{MouseTerminal, TermRead};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Paragraph, Text, Widget};
use tui::{Frame, Terminal};

type Backend = TermionBackend<AlternateScreen<MouseTerminal<RawTerminal<Stdout>>>>;

pub struct Gui<'a> {
    terminal: Terminal<Backend>,
    actions: HashMap<Key, Box<dyn FnMut() -> Control<'a> + 'a>>,
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
        F: FnMut() -> Control<'a> + 'a,
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

        let mut keys = io::stdin().keys();

        terminal.draw(|f| draw(f))?;
        while let Some(key) = keys.next() {
            let key = key?;
            if let Key::Esc = key {
                return Ok(0);
            }

            if let Some(action) = actions.get_mut(&key) {
                match action() {
                    Control::Continue => {
                        terminal.draw(|f| draw(f))?;
                        continue;
                    }
                    Control::Input(callback) => {
                        let mut input = String::new();
                        loop {
                            terminal.draw(|mut f| {
                                let size = f.size();
                                Paragraph::new([Text::raw(&input)].iter())
                                    .style(Style::default().fg(Color::Yellow))
                                    .block(Block::default().borders(Borders::ALL).title("Input"))
                                    .render(&mut f, center(size, (18, 3)));
                            })?;

                            match keys.next().unwrap()? {
                                Key::Esc => return Ok(0),
                                Key::Char('\n') => break,
                                Key::Char(c) => input.push(c),
                                Key::Backspace => {
                                    input.pop();
                                }
                                _ => continue,
                            };
                        }

                        callback(input);
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

fn center(size: Rect, (w, h): (u16, u16)) -> Rect {
    let (w, h) = (cmp::min(size.width, w), cmp::min(size.height, h));
    let x = size.x + ((size.width - w) / 2);
    let y = size.y + ((size.height - h) / 2);

    Rect::new(x, y, w, h)
}

pub enum Control<'a> {
    Continue,
    Input(Box<dyn Fn(String) + 'a>),
    Quit(i32),
}
