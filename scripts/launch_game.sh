#!/bin/sh
# launch_game.sh - LGTUI Game Launcher Script
# Launches a Windows executable using the specified Wine/Proton runner,
# custom WINEPREFIX, and runtime wrapper configurations (MangoHud, GameMode, DXVK, VKD3D).

# Enable setup flags if configured
if [ "$DXVK" = "1" ]; then
    export setup_dxvk=1
    echo "[LAUNCHER] Enabling DXVK overlay/wrapper"
fi

if [ "$VKD3D" = "1" ]; then
    export setup_vkd3d=1
    echo "[LAUNCHER] Enabling VKD3D-Proton wrapper"
fi

# Set the WINEPREFIX env var
if [ -n "$WINEPREFIX" ]; then
    export WINEPREFIX
    echo "[LAUNCHER] Using Wine Prefix: $WINEPREFIX"
fi

# Fallback to system default wine if no runner specified
RUNNER="${RUNNER:-/usr/bin/wine}"
echo "[LAUNCHER] Runner: $RUNNER"
echo "[LAUNCHER] Executable: $EXEC_PATH"
if [ -n "$GAME_ARGS" ]; then
    echo "[LAUNCHER] Arguments: $GAME_ARGS"
fi

# Build wrapper command prefix depending on MangoHud and GameMode
CMD=""
if [ "$GAMEMODE" = "1" ]; then
    if command -v gamemoderun >/dev/null 2>&1; then
        CMD="gamemoderun"
        echo "[LAUNCHER] Prepending GameMode wrapper"
    else
        echo "[LAUNCHER] [WARNING] gamemoderun not found on system PATH!"
    fi
fi

if [ "$MANGOHUD" = "1" ]; then
    if command -v mangohud >/dev/null 2>&1; then
        if [ -n "$CMD" ]; then
            CMD="$CMD mangohud"
        else
            CMD="mangohud"
        fi
        echo "[LAUNCHER] Prepending MangoHud wrapper"
    else
        echo "[LAUNCHER] [WARNING] mangohud not found on system PATH!"
    fi
fi

# Execute the runner with game executable and args
if [ -n "$CMD" ]; then
    # Parse wrappers using eval to handle multiple commands correctly
    eval exec $CMD '"$RUNNER"' '"$EXEC_PATH"' $GAME_ARGS
else
    exec "$RUNNER" "$EXEC_PATH" $GAME_ARGS
fi
