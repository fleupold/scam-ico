mod context;
mod contract;
mod gui;
mod truffle;
mod wallet;

use crate::context::{Context, State};
use crate::gui::{Control, Gui};
use crate::wallet::Wallet;
use bip39::{Language, Mnemonic};
use std::cell::RefCell;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;
use termion::event::Key;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Paragraph, SelectableList, Text, Widget};
use web3::futures::Future;
use web3::transports::Http;
use web3::types::Address;
use web3::Web3;

#[derive(Debug, StructOpt)]
#[structopt(name = "scam-ico", about = "Scam ICO Client.")]
struct Opt {
    /// The web3 transport to use.
    #[structopt(short, long, default_value = "http://localhost:7545")]
    transport: String,

    /// Path to truffle project.
    #[structopt(short = "p", long, default_value = ".")]
    truffle_project: PathBuf,

    /// The Scam ICO contract address. If it is not specified the address in the
    /// truffle artifact will be used.
    #[structopt(short, long)]
    contract: Option<Address>,

    /// The BIP-0039 mnemonic to use for generating BIP-0043 accounts. If it is
    /// not specified then it will use web3 to get the list of accounts and for
    /// signing.
    #[structopt(short, long)]
    mnemonic: Option<MnemonicArg>,

    /// The number of accounts to generating when using a mnemonic. Has no effect
    /// otherwise
    #[structopt(long, default_value = "3")]
    accounts: usize,
}

#[derive(Debug)]
pub struct MnemonicArg(Mnemonic);

impl FromStr for MnemonicArg {
    type Err = Box<dyn Error + 'static>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mnemonic = Mnemonic::from_phrase(s, Language::English)?;
        Ok(MnemonicArg(mnemonic))
    }
}

impl MnemonicArg {
    fn as_inner(&self) -> Mnemonic {
        self.0.clone()
    }
}

fn main() {
    let opt = Opt::from_args();

    let (eloop, http) = Http::new(&opt.transport).expect("error setting up transport");
    eloop.into_remote();
    let web3 = Web3::new(http);

    let wallet = if let Some(mnemonic) = &opt.mnemonic {
        Wallet::with_mnemonic(mnemonic.as_inner(), opt.accounts)
    } else {
        Wallet::local(web3.clone())
            .wait()
            .expect("failed to get local accounts")
    };

    let context = if let Some(ico_address) = opt.contract {
        Context::with_ico_address(web3.clone(), &opt.truffle_project, ico_address)
            .wait()
            .expect("failed to load context as specified address")
    } else {
        Context::new(web3.clone(), &opt.truffle_project)
            .wait()
            .expect("failed to deploy ico contract and load context")
    };

    let account_selection = RefCell::new(0usize);
    let naccounts = wallet.accounts().count();

    use Control::*;
    Gui::new()
        .expect("failed to setup terminal")
        .with_action(Key::Char('q'), || Quit(0))
        .with_action(Key::F(5), || Continue)
        .with_action(Key::Up, || {
            account_selection.replace_with(|&mut v| match v {
                0 => naccounts - 1,
                n => n - 1,
            });
            Continue
        })
        .with_action(Key::Down, || {
            account_selection.replace_with(|&mut v| (v + 1) % naccounts);
            Continue
        })
        .with_action(Key::Char('s'), || Input(Box::new(|input| {
            let amount: f64 = match input.parse() {
                Ok(a) => a,
                Err(_) => return,
            };
            let account = wallet.accounts().nth(*account_selection.borrow()).unwrap();
            let _ = context.purchase_weth(account, amount).wait();
        })))
        .with_action(Key::Char('d'), || Input(Box::new(|input| {
            let amount: f64 = match input.parse() {
                Ok(a) => a,
                Err(_) => return,
            };
            let account = wallet.accounts().nth(*account_selection.borrow()).unwrap();
            let _ = context.magic_weth(account, amount).wait();
        })))
        .with_action(Key::Char('f'), || Input(Box::new(|input| {
            let amount: f64 = match input.parse() {
                Ok(a) => a,
                Err(_) => return,
            };
            let account = wallet.accounts().nth(*account_selection.borrow()).unwrap();
            let _ = context.fund(account, amount).wait();
        })))
        .with_action(Key::Char('c'), || {
            let account = wallet.accounts().nth(*account_selection.borrow()).unwrap();
            let _ = context.claim(account).wait();
            Continue
        })
        .run(|mut f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(&[Constraint::Length(8), Constraint::Min(0), Constraint::Length(4)][..])
                .split(size);

            Paragraph::new([
                    Text::raw("\nOnce in a lifetime chance to get rich!\n"),
                    Text::raw("Participate in our ICO and receive 10 times what you contributed in just 2 hours!\n\n"),
                    Text::raw(match context.state().wait() {
                        Ok(State::Funding(remaining)) => format!("Only {} left!", remaining),
                        Ok(State::Closed) => "ICO closed, come back soon to claim your mullah!".to_string(),
                        Ok(State::Finished) => "Claim your rewards now!".to_string(),
                        Err(_) => "???".to_string(),
                    })
                ].iter())
                .wrap(true)
                .alignment(Alignment::Center)
                .block(Block::default().title("Scam ICO").borders(Borders::ALL))
                .render(&mut f, chunks[0]);

            let accounts: Vec<_> = wallet.accounts()
                .map(|account| {
                    let (eth, weth, contrib, scm) = context.balances(account).wait().unwrap_or((-1.0, -1.0, -1.0, -1.0));
                    format!("{:?} {:7.2} ETH | {:6.2}>{:6.2} WETH | {:7.2} SCM", account, eth, weth, contrib, scm)
                })
                .collect();
            SelectableList::default()
                .items(&accounts)
                .select(Some(*account_selection.borrow()))
                .highlight_style(
                    Style::default()
                        .modifier(Modifier::ITALIC)
                        .fg(Color::Yellow),
                )
                .highlight_symbol(">")
                .block(Block::default().title("Accounts").borders(Borders::ALL))
                .render(&mut f, chunks[1]);

            Paragraph::new([
                    Text::styled("q", Style::default().modifier(Modifier::BOLD)),
                    Text::raw(": Quit                   "),
                    Text::styled("F5", Style::default().modifier(Modifier::BOLD)),
                    Text::raw(": Refresh View          "),
                    Text::styled("^/v", Style::default().modifier(Modifier::BOLD)),
                    Text::raw(": Select Account\n"),
                    Text::styled("s", Style::default().modifier(Modifier::BOLD)),
                    Text::raw(": Purchase WETH          "),
                    Text::styled("d", Style::default().modifier(Modifier::BOLD)),
                    Text::raw(": Magic WETH (testnet)   "),
                    Text::styled("f", Style::default().modifier(Modifier::BOLD)),
                    Text::raw(": Participate in ICO     "),
                    Text::styled("c", Style::default().modifier(Modifier::BOLD)),
                    Text::raw(": Claim SCM"),
                ].iter())
                .wrap(true)
                .alignment(Alignment::Left)
                .block(Block::default().title("Help").borders(Borders::ALL))
                .render(&mut f, chunks[2]);
        })
        .unwrap();
}
