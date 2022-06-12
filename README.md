# Custom scripts

A collection of custom Rust scripts for personal usage.

Run `./install.sh` to install all. This deploys:

- Rust scripts to `$CARGO_HOME/bin/`
- Shell scripts to `~/.bin/`

## Dependencies

- `scrot` for `bin/screenlock`
- `iwconfig` for `bin/netinfo` (pkg: `wireless_tools`)
- `pw-dump` for `bin/fix_xonar_output` and `bin/change_sink`

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
