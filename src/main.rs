mod context;
mod contract;
mod gui;
mod truffle;
mod wallet;

use crate::context::Context;
use crate::gui::{Control, Gui};
use crate::wallet::Wallet;
use bip39::{Language, Mnemonic};
use std::cell::RefCell;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;
use termion::event::Key;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, SelectableList, Widget};
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
    let account_addresses: Vec<_> = wallet
        .accounts()
        .map(|address| format!("{:?}", address))
        .collect();

    use Control::*;
    Gui::new()
        .expect("failed to setup terminal")
        .with_action(Key::F(5), || Continue)
        .with_action(Key::Char('q'), || Quit(0))
        .with_action(Key::Up, || {
            account_selection.replace_with(|&mut v| match v {
                0 => account_addresses.len() - 1,
                n => n - 1,
            });
            Continue
        })
        .with_action(Key::Down, || {
            account_selection.replace_with(|&mut v| (v + 1) % account_addresses.len());
            Continue
        })
        .run(|mut f| {
            let size = f.size();
            SelectableList::default()
                .items(&account_addresses)
                .select(Some(*account_selection.borrow()))
                .highlight_style(
                    Style::default()
                        .modifier(Modifier::ITALIC)
                        .fg(Color::Yellow),
                )
                .highlight_symbol(">")
                .block(Block::default().title("Accounts").borders(Borders::ALL))
                .render(&mut f, size);
        })
        .unwrap();
}
