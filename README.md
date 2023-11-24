# Custom scripts

A collection of custom Rust scripts for personal usage.

Run `./install.sh` to install all. This deploys:

- Rust scripts to `$CARGO_HOME/bin/`
- Shell scripts to `~/.bin/`

## Dependencies

- `rust` to compile the rust scripts
- `scrot` for `bin/screenlock`
- `iwconfig` for `netinfo` (pkg: `wireless_tools`)
- `iw` for `netinfo`
- `pw-dump` for `bin/fix_xonar_output` and `bin/change_sink`

## Installation

### Via Script

There's the `./install.sh` script, which does all of the work for you.

1. It copies all shell scripts into your `~/.bin` folder.
1. It compiles the rust scripts and copies them over to your `~/.bin` folder.
1. Make sure to add your `~/.bin` to your path.

If you want to adjust the target directory (`~/.bin`), update the `BIN_FOLDER` variable in the `install.sh` script.

### Manual installation

For the shell scripts:
- Just copy any script from the `./shell` folder you like to your target directory.

For the rust code:

Option 1:

- Run `cargo install --path ./`
- Add the `$CARGO_HOME/bin` to your `$PATH`, which by default is `~/.cargo/bin`.

Option 2:

- Run `cargo build --locked --release`
- Copy the binaries you want from the `./target/release/` folder to your target directory.

## Git Hooks

There're two hooks, which automatically deploy the project when pulling new commits.

Great for syncing changes between multiple machines.

## Screenlock

Screenlock trigger on sleep via a `systemd` service looks like this:

```
[Unit]
Description=Lock the screen
Before=sleep.target

[Service]
User=%i
Group=%i
Type=forking
Environment=DISPLAY=:0
ExecStart=/home/%i/.cache/cargo/bin/blur 5 -vvv

[Install]
WantedBy=sleep.target
```
