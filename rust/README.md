# Selfhosted Game Server

A collection of management scripts and configs for my game servers.

## Management

Use `misc/start.sh` to start all games, uncomment unwanted games.
The games will be launched in respective tmux sessions.

Use `misc/stop.sh` to stop all games and to backup all servers.
To backup maps on demand call the respective script.
For Factorio this is `./scripts/factorio.sh backup`.
For Minecraft it's `./scripts/minecraft.sh minecraft backup` and `./scripts/minecraft.sh ftb backup`

## Updates

To update factorio, place the `factorio_headless_x64_*` file into your home and call `./scripts/factorio update`.
