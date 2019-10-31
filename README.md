# Scam ICO

Scam ICO is my take on the
[Scam Coin ICO](https://drive.google.com/open?id=1HagdvQApJeS2NtdlOkweZV8sQU8KBJ3PsQgmXSpCia0)
onboarding exercise.

## Usage

### Requirements

- Ganache 2.x (running on `localhost:7545`)
- NodeJS 10.x
- Rust 1.38

### Building and Running

The first thing you need to do in order to try it out is to build and deploy the
smart contracts. You can do this with truffle by calling the following script:

```
$ npm install
$ npm run deploy
```

Next, you can use the tui interface to interact with the contract:

```
$ cargo run
```

## TODO:

- [ ] Contract unit tests
- [ ] Rinkeby network
  - Requires getting account private keys for signing
  - Refactoring the `Context` struct to be aware of these accounts in order for
    it to sign transactions offline before sending them off
- [ ] Gas estimation with `gas-station`
