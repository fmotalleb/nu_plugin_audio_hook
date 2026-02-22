# nu_plugin_audio_hook

A [Nushell](https://www.nushell.sh/) plugin for generating and playing sounds. Supports beeping, tone generation, metadata manipulation, and playback for multiple audio formats.

---

## Features

- **`sound beep`** — Play a simple beep sound.
- **`sound make`** — Generate a noise with a given frequency and duration.
- **`sound meta`** — Retrieve metadata (duration, artist, album, etc.) from an audio file.
- **`sound meta set`** — Modify metadata tags in an audio file using format-agnostic key names.
- **`sound play`** — Play an audio file with a live progress display, interactive controls, and volume adjustment. By default supports FLAC, WAV, MP3, and OGG. Use the `all-decoders` feature to enable AAC and MP4 playback.

---

## Usage

### Generate a simple noise

```bash
sound make 1000 200ms
```

### Generate a noise sequence

```bash
[ 300.0, 500.0, 1000.0, 400.0, 600.0 ] | each { |it| sound make $it 150ms }
```

### Generate a noise with 50% volume

```bash
sound make 1000 200ms -a 0.5
```

### Save a generated tone to a file

```bash
sound make 1000 200ms --data | save --raw output.wav
```

### Play an audio file (first 3 seconds only)

```bash
sound play audio.mp3 -d 3sec
```

### Play an audio file starting at 2x volume

```bash
sound play audio.mp3 -a 2.0
```

### Play an audio file starting at 50% volume

```bash
sound play audio.mp3 -a 0.5
```

### Play silently — no terminal output (for scripting or background use)

```bash
sound play audio.mp3 --no-progress
```

### Play with Nerd Font icons

```bash
sound play audio.mp3 --nerd-fonts
```

### Retrieve metadata from an audio file

```bash
sound meta audio.mp3
```

Example output:

```nushell
╭───────────────┬────────────────────────────╮
│ size          │ 6.4 MiB                    │
│ format        │ mp3                        │
│ bitrate       │ 320                        │
│ audio_bitrate │ 320                        │
│ artist        │ SINGER                     │
│ title         │ TITLE                      │
│ album         │ ALBUM                      │
│ albumartist   │ SINGER                     │
│ comment       │ Tagged with MusicBrainz    │
│ date          │ 2024-03-15                 │
│ genre         │ Rock                       │
│ track_no      │ 1                          │
│ total_tracks  │ 12                         │
│ artwork       │ [list 1 item]              │
│ duration      │ 4:05                       │
│ sample_rate   │ 44100                      │
│ channels      │ 2                          │
╰───────────────┴────────────────────────────╯
```

The `artwork` field is a list of records, one per embedded image:

```nushell
sound meta audio.mp3 | get artwork
# ╭───┬───────────────┬────────────┬──────────╮
# │ # │ pic_type      │ mime_type  │ size     │
# ├───┼───────────────┼────────────┼──────────┤
# │ 0 │ CoverFront    │ image/jpeg │ 127.3 KB │
# ╰───┴───────────────┴────────────┴──────────╯
```

FLAC and lossless files additionally expose `bit_depth`:

```nushell
sound meta audio.flac | select size format bitrate bit_depth
# ╭───────────┬──────────╮
# │ size      │ 42.3 MiB │
# │ format    │ flac     │
# │ bitrate   │ 1411     │
# │ bit_depth │ 24       │
# ╰───────────┴──────────╯
```

### Modify metadata (change the artist tag)

```bash
sound meta set audio.mp3 -k artist -v "new-artist"
```

Key names are **case-insensitive** — `artist`, `Artist`, and `ARTIST` all work. Key names are format-agnostic — the same key works across MP3, FLAC, OGG, and MP4 files. Use `sound meta --all` to list every available key name.

### Set a comment tag

```bash
sound meta set audio.mp3 -k comment -v "ripped from vinyl"
```

### Set ReplayGain values

```bash
sound meta set audio.mp3 -k replaygain_track_gain -v "-6.2 dB"
sound meta set audio.mp3 -k replaygain_track_peak -v "0.998"
```

### List all available metadata key names

```bash
sound meta --all
```

Key names are normalised to lowercase before lookup, so `Artist`, `ARTIST`, and `artist` are all accepted. The table below shows every supported key grouped by category.

#### Core identity

| Key | Maps to |
| --- | --- |
| `album` | Album title |
| `albumartist` | Album-level artist |
| `albumsortorder` | Album title sort order |
| `artist` | Track artist |
| `artistsortorder` | Track artist sort order |
| `title` | Track title |
| `titlesortorder` | Track title sort order |
| `subtitle` | Track subtitle |
| `setsubtitle` | Set/disc subtitle |

#### People & roles

| Key | Maps to |
| --- | --- |
| `composer` | Composer |
| `composersortorder` | Composer sort order |
| `conductor` | Conductor |
| `label` | Record label |
| `lyricist` | Lyricist |
| `movement` | Movement name |
| `movementnumber` | Movement number |
| `movementtotal` | Total movements |
| `organization` | Publisher |
| `producer` | Producer |
| `publisher` | Publisher (alias for `organization`) |
| `remixer` | Remixer / mix artist |
| `work` | Work title |

#### Dates

| Key | Maps to |
| --- | --- |
| `date` | Recording date (ISO 8601, e.g. `2024-03-15`) |
| `originalyear` | Original release date |
| `releasedate` | Release date |
| `year` | Release year (bare integer) |

#### Identifiers

| Key | Maps to |
| --- | --- |
| `barcode` | Release barcode (EAN/UPC) |
| `cataloguenumber` | Catalogue number |
| `isrc` | ISRC |

#### Style & content

| Key | Maps to |
| --- | --- |
| `bpm` | BPM (decimal string) |
| `comment` | Comment |
| `compilation` | Compilation flag (`1` / `0`) |
| `copyright` | Copyright message |
| `discnumber` | Disc number |
| `encodedby` | Encoded by |
| `encodingsettings` | Encoder settings |
| `genre` | Genre |
| `grouping` | Content group / grouping |
| `initialkey` | Initial key (e.g. `Am`) |
| `language` | Language |
| `lyrics` | Lyrics text |
| `mood` | Mood |
| `originalalbum` | Original album title |
| `originalartist` | Original artist |
| `script` | Script (e.g. `Latin`) |
| `track` | Track number |

#### ReplayGain

| Key | Maps to |
| --- | --- |
| `replaygain_album_gain` | Album gain (dB string, e.g. `-6.5 dB`) |
| `replaygain_album_peak` | Album peak (float string, e.g. `0.998`) |
| `replaygain_track_gain` | Track gain |
| `replaygain_track_peak` | Track peak |

---

## Live Playback Display

When playing a file, `sound play` renders a live progress bar to stderr:

```nushell
▶  0:42 / 4:05  [██████████░░░░░░░░░░░░░░░░░░░░]  17%  🔊 [████████░░░░░░] 100%
```

Because the display writes to stderr, stdout remains clean — piping the result of `sound play` to another command works without any garbled output. Use `--no-progress` (`-q`) to suppress the display entirely for scripting or background use.

### Nerd Font mode

If you have a [Nerd Font](https://www.nerdfonts.com) installed and configured in your terminal, pass `--nerd-fonts` (`-n`) or set `NERD_FONTS=1` in your environment for richer icons:

```nushell
  0:42 / 4:05  [██████████░░░░░░░░░░░░░░░░░░░░]  17%   [████████░░░░░░] 100%
```

To enable permanently, add this to your `env.nu`:

```nushell
$env.NERD_FONTS = "1"
```

---

## Interactive Controls

For files longer than **1 minute**, interactive keyboard controls are enabled automatically:

| Key | Action |
| --- | --- |
| `Space` | Play / pause |
| `→` or `l` | Seek forward 5 seconds |
| `←` or `h` | Seek backward 5 seconds |
| `↑` or `k` | Volume up 5% |
| `↓` or `j` | Volume down 5% |
| `m` | Toggle mute |
| `q` or `Esc` | Stop and quit |

The control hint is shown inline on the progress bar and updates live to reflect the current state:

```nushell
▶  0:42 / 4:05  [██████████░░░░░░░░░░░░░░░░░░░░]  17%  🔊 [████████░░░░░░] 100%  « [SPACE/pause] »  [↑↓/kj] vol  [m] mute  [q] quit
```

Use `--no-progress` to disable all terminal output and controls, which is recommended when running in the background or piping output.

---

## Installation

### Linux: install ALSA development package

#### Debian / Ubuntu

```bash
sudo apt update
sudo apt install -y libasound2-dev pkg-config
```

#### RHEL / CentOS / Rocky / Alma

```bash
sudo dnf install -y alsa-lib-devel pkgconf-pkg-config
```

#### Arch Linux

```bash
sudo pacman -S --needed alsa-lib pkgconf
```

#### openSUSE

```bash
sudo zypper install alsa-lib-devel pkg-config
```

### Recommended: using [nupm](https://github.com/nushell/nupm)

```bash
git clone https://github.com/FMotalleb/nu_plugin_audio_hook.git
nupm install --path nu_plugin_audio_hook -f
```

### Manual compilation

```bash
git clone https://github.com/FMotalleb/nu_plugin_audio_hook.git
cd nu_plugin_audio_hook
cargo build -r --locked --features=all-decoders
plugin add target/release/nu_plugin_audio_hook
```

### Install via Cargo (git)

```bash
cargo install --git https://github.com/FMotalleb/nu_plugin_audio_hook.git --locked --features=all-decoders
plugin add ~/.cargo/bin/nu_plugin_audio_hook
```

### Install via Cargo (crates.io) — not recommended

> Since I live in Iran and crates.io often restricts package updates, the version there might be outdated.

```bash
cargo install nu_plugin_audio_hook --locked --features=all-decoders
plugin add ~/.cargo/bin/nu_plugin_audio_hook
```

---

## Supported formats

### Default install

Enabled out of the box with no extra flags:

| Format | Feature flag | Notes |
| --- | --- | --- |
| MP3 | `symphonia-mp3` | Via Symphonia; better accuracy than minimp3 |
| FLAC | `flac` | Lossless compression |
| OGG Vorbis | `vorbis` | Open lossy format |
| WAV | `wav` | Uncompressed PCM |

### With `--features=all-decoders` (recommended)

Everything above plus:

| Format | Feature flag | Notes |
| --- | --- | --- |
| AAC | `symphonia-aac` | Used by Apple, YouTube, most streaming services |
| MP4 / M4A | `symphonia-isomp4` | Container for AAC and ALAC |
| ALAC | `symphonia-all` | Apple Lossless; only available via bundle |
| ADPCM | `symphonia-all` | Adaptive PCM; common in games |
| CAF | `symphonia-all` | Core Audio Format; Apple professional audio |
| MKV / WebM (Opus) | `symphonia-all` | Open container with Opus codec |
| MP3 (minimp3) | `minimp3` | Lightweight alternative MP3 decoder |
| FLAC (Symphonia) | `symphonia-flac` | Alternative FLAC decoder |
| OGG (Symphonia) | `symphonia-vorbis` | Alternative Vorbis decoder |
| WAV (Symphonia) | `symphonia-wav` | Alternative WAV decoder |

> **Note:** ALAC, ADPCM, CAF, and MKV/Opus are only available through the
> `symphonia-all` bundle. rodio 0.21 does not expose them as individual feature
> flags. All other formats can be opted into selectively.

### Compile with specific formats only

```bash
# MP3 + AAC + MP4 only
cargo build -r --locked --features=symphonia-mp3,symphonia-aac,symphonia-isomp4

# Everything
cargo build -r --locked --features=all-decoders
```

---

## Contributors

See [CONTRIBUTORS.md](CONTRIBUTORS.md) for the full list of contributors.
