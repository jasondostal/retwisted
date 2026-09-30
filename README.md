# retwisted

A faithful remake of **After Dark: Totally Twisted** (Berkeley Systems, 1995) for
modern macOS. It runs as a real macOS screen saver, and as a window on your desktop.
All 13 modules are here: Bungee Roulette, Chameleon, Coming Soon!, Flying Toilets,
FrankenScreen, Message Mayhem, Mike's So-called Life, Mime Hunt, Mowin' Boris,
Phlegm Boy, Shock Clocks, Toxic Swamp and Voyeur.

**This repository contains no Berkeley Systems art, sound or text.** You supply your
own copy of the original software. retwisted reads it and makes the files that it
needs on your computer. This is the same model that ScummVM uses.

## Quick start

These steps install the screen saver. They take approximately 10 minutes.

### 1. Get the original software

Download ONE of these files. Each file contains the full Mac release.

| Source | File | Release |
|---|---|---|
| [Macintosh Garden](https://macintoshgarden.org/apps/after-dark-totally-twisted) | `After_Dark_-_Totally_Twisted.sit` (2.1 MB) | Floppy |
| [Macintosh Garden](https://macintoshgarden.org/apps/after-dark-totally-twisted) | `totallytwistedcd.sit` (22.8 MB) | CD |
| [Internet Archive](https://archive.org/details/TTW320CD) | `TTW320CD.ISO` (37.9 MB) | CD |
| [Macintosh Repository](https://www.macintoshrepository.org/1511-after-dark-totally-twisted) | the same two `.sit` files | Floppy, CD |

> **NOTE:** Do not use the `FlyingToiletsDemo` file. It contains only one module.
> Do not use `TTW320CD.zip`. retwisted cannot read `.zip` files.

> **NOTE:** Do not expand the file. retwisted reads the `.sit` or `.ISO` file directly.

### 2. Install the build tools

You need a Mac with macOS 13 or later. Open the Terminal app.

1. Install the Apple command line tools:

   ```
   xcode-select --install
   ```

2. Install Rust:

   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Close Terminal. Then open Terminal again.

### 3. Build and install the screen saver

1. Get the code:

   ```
   git clone https://github.com/jasondostal/retwisted.git
   cd retwisted
   ```

2. Build and install:

   ```
   tools/build_saver.sh --no-packs --install
   ```

> **CAUTION:** This command closes System Settings. Save your work in System Settings first.

### 4. Connect the screen saver to your file

1. Open **System Settings > Screen Saver**.
2. Select **Retwisted**. It is at the end of the list.
3. Click **Options…**.
4. Click **Locate…**.
5. Select the file from step 1. Then click **Open**.
6. Wait approximately 5 seconds. The module list becomes active.
7. Select a module. Then click **OK**.

The screen saver is ready. To see it now, click **Preview**.

> **NOTE:** You do these steps one time only. To use a different file, click **Locate…** again.

> **NOTE:** Select **Randomizer** at the top of the list to show a different module at intervals.

## Run a module in a window

You can also run a module in a window on your desktop.

1. Make the module files from your download:

   ```
   cargo run --release -p ripper --bin twistedrip -- ~/Downloads/After_Dark_-_Totally_Twisted.sit --out assets
   ```

   Replace `~/Downloads/After_Dark_-_Totally_Twisted.sit` with the location of your file.

2. Start a module:

   ```
   cargo run --release -p app -- mime-hunt
   ```

Use these keys in the window:

| Key | Action |
|---|---|
| `[` and `]` | Show the previous or the next module |
| `1` to `9` | Select a module setting. The window title shows the setting. |
| `-` and `=` | Decrease or increase the selected setting |
| `,` and `.` | Decrease or increase the volume |
| Caps Lock | Turn the Caps Lock effect on or off (Mime Hunt: you control the gun) |

## Problems

| Problem | Solution |
|---|---|
| The **Options…** window does not open. | Close System Settings. Then open it again. |
| **Locate…** does not show the file dialog. | Make the folder `/Users/Shared/Retwisted`. Put your file in it. Open **Options…** again. Click **Use Shared Folder**. |
| retwisted says "That file isn't a Totally Twisted download". | Use a file from the table in step 1. |
| `tools/build_saver.sh` says "command not found" or `cargo` is missing. | Do step 2 again. Then close and open Terminal. |

## About

retwisted is a fan project for preservation. It is not affiliated with or endorsed by
Berkeley Systems, Sierra or their successors. "After Dark" and "Totally Twisted" are
trademarks of their owners. The download sites above are third-party archives. Make
sure that you can legally use the software in your country.

retwisted is licensed under the [MIT License](LICENSE). The license covers the
retwisted code only. It gives no rights to the original After Dark software.
`ripper/src/container/sitcodec.rs` contains code from the `stuffit` crate
(MIT License, Copyright (c) 2025 Ben Letchford).

The behavior of each module comes from a study of the original program. The colors
use the Macintosh display gamma, so they look as they did on a 1995 Macintosh monitor.

## For developers

- `engine/` — sprite model, palettes, clocks, the Berkeley random number generators, and the module interface
- `app/` — the 13 modules (`app/src/modules/`), the shared draw code, the window app, and `bin/frame` (makes a PNG with no window)
- `saver/` — the C interface to the modules and the macOS `ScreenSaverView` (`saver/macos/`)
- `ripper/` — `twistedrip`, which reads the original download and makes the module files
- `tools/` — `build_saver.sh`, headless test tools for the screen saver, the motion linter, and the older Python ripper

Run the tests:

```
cargo test --workspace --release
```

More details: [`docs/saver.md`](docs/saver.md) (the screen saver and its macOS problems),
[`docs/ripper.md`](docs/ripper.md), [`docs/motion-lint.md`](docs/motion-lint.md).
