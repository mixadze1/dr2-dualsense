# dr2-dualsense

Adaptive triggers and haptics for the **DualSense** controller in **DiRT Rally 2.0** (PC, Steam).

DiRT Rally 2.0 does not support the DualSense. This mod is a drop-in `xinput9_1_0.dll` placed in the game folder, which the game loads by itself. The mod:

- reads the DualSense directly (USB or Bluetooth),
- presents it to the game as an Xbox controller,
- drives the triggers and vibration from the game's UDP telemetry.

No game files are modified. Delete the DLL and everything is back to stock.

## Features

| | Effect |
|---|---|
| **L2 (brake)** | Progressive resistance that gets firmer the deeper you press. When the front wheels lock, the trigger pulses like ABS. |
| **R2 (throttle)** | Firm resistance. It vibrates during wheelspin, with strength following the amount of slip, and pulses at the rev limiter. |
| **Vibration** | Road texture (gravel, bumps) from suspension velocities, landings and impacts, gear-shift kicks, wheel slip, plus the game's own rumble. |
| **Lightbar** | Rev meter going green → yellow → red, flashing at the limiter. |

Vibration is quiet when you don't want it:
- **Off throttle:** body vibration fades out within about 0.1 s when you release R2 (`throttle_gate`).
- **Standing still without pressing the pedals:** no vibration (`quiet_when_stopped`). Revving in place still lets you feel the limiter.
- **Game paused:** detected from frozen telemetry. Triggers are released and vibration stops (`pause_detection`).
- **Menus and replays:** triggers are free and there is no vibration.

### In-game controls
- **PS + D-pad Up/Down:** effect intensity. The player LEDs show the level.
- **PS + Options:** toggle the mod on or off. A green or red flash confirms.

## Installation

1. Download `dr2-dualsense-vX.Y.Z.zip` from [Releases](../../releases), or build it yourself with `cargo build --release`.
2. Close the game and run:
   ```powershell
   powershell -ExecutionPolicy Bypass -File install.ps1
   ```
   The script:
   - finds the game in your Steam libraries (or pass `-GameDir`) and copies the DLL;
   - enables telemetry in `Documents\My Games\DiRT Rally 2.0\hardwaresettings\hardware_settings_config.xml` by setting `<udp enabled="true" extradata="3" port="20777" .../>`;
   - keeps a `.dr2ds-backup` copy of that file next to it.
3. **Steam → DiRT Rally 2.0 → Properties → Controller → "Disable Steam Input".** Otherwise Steam grabs the controller.
4. Close DS4Windows, DSX and similar tools.
5. Start the game. The game sees an Xbox controller: that is your DualSense going through the mod.

To uninstall, run `uninstall.ps1`. It also restores the telemetry setting.

## Configuration

On first start the mod creates `dr2_dualsense.ini` in the game folder. The file is reloaded live, so there is no need to restart the game.

The main settings are a preset and two power levels:

```ini
[general]
preset = medium          ; soft | medium | strong
trigger_power = 100      ; trigger strength, % on top of the preset
vibration_power = 55     ; vibration strength, %
```

| Preset | Brake L2 | Throttle R2 | Vibration | For |
|---|---|---|---|---|
| `soft` | 1→4 | 1→3 | 35% | Long sessions, gentle |
| `medium` | 1→6 | 2→4 | 55% | Default |
| `strong` | 2→8 | 2→5 | 85% | Maximum |

Values are resistance levels from 0 to 8, going from the start to the end of the trigger travel.

Every other setting is listed in the file, commented out. Remove the leading `;` to override the preset. These include:
- trigger zones;
- lockup and wheelspin thresholds;
- vibration frequencies;
- road, impact and shift mix;
- `coast_vibration` (how much vibration remains off throttle);
- `smoothing_ms` (how long vibration takes to fade).

If SimHub or another tool already listens on port 20777, set `udp_forward_port` and the mod will pass telemetry packets on to it.

Log: `dr2_dualsense.log` in the game folder. It shows when a stage starts, when the game is paused and when you are in menus.

## Testing without the game

```powershell
cargo build --release
.\target\release\dr2ds-harness.exe --seconds 60
.\target\release\dr2ds-sim.exe
```

- `dr2ds-harness` loads the DLL the same way the game does and prints button and stick input.
- `dr2ds-sim`, run in a second window, sends a scripted stage over UDP: cruise, gravel, wheelspin launch, rev limiter, upshifts, front lockup, a jump, a pause.

You should feel each phase, and the controller must go quiet during the pause.

## Tests

```powershell
cargo build
cargo test
```

Covered:
- trigger effect encoding;
- USB and Bluetooth HID output reports with CRC32;
- input report parsing;
- the Codemasters telemetry layout;
- effect logic: lockup, wheelspin, limiter, impacts, shifts, road, pause, throttle gate, standing still;
- presets and power scaling;
- XInput mapping and routing;
- PS combos;
- config parsing;
- the UDP receiver;
- the DLL's exports.

## Limitations

- Vibration uses the DualSense's rumble-compatible mode, which also works over Bluetooth, not the audio-driven haptics channel.
- If the game is killed from Task Manager, the triggers may stay firm. Turn the controller off and on again.

## License

MIT
