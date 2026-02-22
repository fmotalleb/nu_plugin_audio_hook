use crossterm::{
    cursor::{Hide, MoveToColumn, MoveUp, Show},
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{Attribute, SetAttribute},
    terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType},
};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::prelude::Accessor;
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, SyntaxShape, Value};
use rodio::{source::Source, Decoder, OutputStreamBuilder, Sink};

use std::io::{stderr, Write};
use std::time::{Duration, Instant};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{utils::{format_duration, load_file}, Sound};

/// Interval for checking keyboard input.
const KEY_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Interval for updating the progress display (to reduce flicker).
const RENDER_INTERVAL: Duration = Duration::from_millis(500);

/// Amount to seek forward or backward when FF/RWD is pressed.
const SEEK_STEP: Duration = Duration::from_secs(5);

/// Minimum duration for interactive controls to be shown.
const CONTROLS_THRESHOLD: Duration = Duration::from_secs(60);

/// How much to change volume per keypress (5%).
const VOLUME_STEP: f32 = 0.05;

/// Maximum volume (200%).
const VOLUME_MAX: f32 = 2.0;

/// Selects the glyph set used for the live progress display.
///
/// Priority order for resolution: `--nerd-fonts` flag → `NERD_FONTS=1` env var →
/// Unicode (if the terminal locale advertises UTF-8) → ASCII fallback.
#[derive(Clone, Copy, PartialEq)]
enum IconSet {
    /// Nerd Font glyphs — richest, requires a patched font.
    NerdFont,
    /// Standard Unicode block/arrow characters — works on most modern terminals.
    Unicode,
    /// Pure ASCII — works everywhere.
    Ascii,
}

impl IconSet {
    /// Play icon: `▶` / `>`.
    fn play(&self)         -> &'static str { match self { Self::NerdFont => "\u{f04b}", Self::Unicode => "▶",  Self::Ascii => ">"   } }
    /// Pause icon: `⏸` / `||`.
    fn pause(&self)        -> &'static str { match self { Self::NerdFont => "\u{f04c}", Self::Unicode => "⏸", Self::Ascii => "||"  } }
    /// Rewind / seek-back icon: `«` / `<<`.
    fn rewind(&self)       -> &'static str { match self { Self::NerdFont => "\u{f04a}", Self::Unicode => "«",  Self::Ascii => "<<"  } }
    /// Fast-forward / seek-forward icon: `»` / `>>`.
    fn fast_forward(&self) -> &'static str { match self { Self::NerdFont => "\u{f04e}", Self::Unicode => "»",  Self::Ascii => ">>"  } }
    /// Music note / track decoration icon.
    fn music(&self)        -> &'static str { match self { Self::NerdFont => "\u{f001}", Self::Unicode => "♪",  Self::Ascii => "#"   } }
    /// Filled bar segment.
    fn fill(&self)         -> &'static str { match self { Self::NerdFont => "█",        Self::Unicode => "█",  Self::Ascii => "#"   } }
    /// Empty bar segment.
    fn empty(&self)        -> &'static str { match self { Self::NerdFont => "░",        Self::Unicode => "░",  Self::Ascii => "."   } }

    /// Volume icon — three tiers based on level.
    fn volume(&self, level: f32) -> &'static str {
        match self {
            Self::NerdFont => {
                if level == 0.0      { "\u{f026}" } // nf-fa-volume_off
                else if level < 0.5  { "\u{f027}" } // nf-fa-volume_down
                else                 { "\u{f028}" } // nf-fa-volume_up
            }
            Self::Unicode => {
                if level == 0.0     { "🔇" }
                else if level < 0.5 { "🔉" }
                else                { "🔊" }
            }
            Self::Ascii => {
                if level == 0.0     { "[M]" } // muted
                else if level < 0.5 { "[v]" }
                else                { "[V]" }
            }
        }
    }
}

/// Nushell command `sound play` — decodes and plays an audio file with a live
/// progress bar on stderr and optional interactive keyboard controls.
pub struct SoundPlayCmd;
impl SimplePluginCommand for SoundPlayCmd {
    type Plugin = Sound;

    fn name(&self) -> &str {
        "sound play"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::new("sound play")
            .required("File Path", SyntaxShape::Filepath, "file to play")
            .named(
                "duration",
                SyntaxShape::Duration,
                "truncate playback to this duration (default: auto-detected from file headers)",
                Some('d'),
            )
            .named(
                "amplify",
                SyntaxShape::Float,
                "initial volume: 1.0 = normal, 0.5 = half, 2.0 = double (default 1.0)",
                Some('a'),
            )
            .switch(
                "no-progress",
                "disable live playback stats (use when piping or running in background)",
                Some('q'),
            )
            .switch(
                "nerd-fonts",
                "use Nerd Font icons in the progress display (or set NERD_FONTS=1)",
                Some('n'),
            )
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "play a sound and exit after 5min",
                example: "sound play audio.mp4 -d 5min",
                result: None,
            },
            Example {
                description: "play a sound starting at 2x volume",
                example: "sound play audio.mp3 -a 2.0",
                result: None,
            },
            Example {
                description: "play a sound starting at 50% volume",
                example: "sound play audio.mp3 -a 0.5",
                result: None,
            },
            Example {
                description: "play a sound for its detected metadata duration",
                example: "sound play audio.mp3",
                result: None,
            },
            Example {
                description: "play silently — no terminal output (background or pipe use)",
                example: "sound play audio.mp3 --no-progress",
                result: None,
            },
            Example {
                description: "play with Nerd Font icons",
                example: "sound play audio.mp3 --nerd-fonts",
                result: None,
            },
        ]
    }

    fn description(&self) -> &str {
        "play an audio file; by default supports FLAC, WAV, MP3 and OGG files \
        (install with `all-decoders` feature to include AAC and MP4). \
        Displays live playback stats by default; use --no-progress (-q) to suppress \
        output for scripting or background use. Interactive controls (space, arrows) \
        are available for files longer than 1 minute, including volume up/down and 5s seeking. \
        Use --nerd-fonts (-n) or set NERD_FONTS=1 for richer icons."
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, nu_protocol::LabeledError> {
        play_audio(engine, call).map(|_| Value::nothing(call.head))
    }
}

// ---------------------------------------------------------------------------
// Core playback
// ---------------------------------------------------------------------------

/// Opens the default audio output, decodes the file via rodio, and delegates to
/// either [`wait_silent`] or [`wait_with_progress`] depending on `--no-progress`.
///
/// Duration is resolved in priority order: `-d` flag → `source.total_duration()` →
/// `lofty::FileProperties::duration()` → 1-hour safety fallback.
fn play_audio(engine: &EngineInterface, call: &EvaluatedCall) -> Result<(), LabeledError> {
    let (file_span, file, path) = load_file(engine, call)?;

    let mut output_stream = OutputStreamBuilder::open_default_stream().map_err(|err| {
        LabeledError::new(err.to_string()).with_label("audio stream exception", call.head)
    })?;

    output_stream.log_on_drop(false);

    let source = Decoder::try_from(file).map_err(|err| {
        LabeledError::new(err.to_string()).with_label("audio decoder exception", file_span)
    })?;

    // Read the tagged file once; reuse the result for both metadata and duration fallback.
    let tagged_file_res = lofty::read_from_path(&path);
    let (title, artist) = tagged_file_res
        .as_ref()
        .ok()
        .and_then(|tf| tf.primary_tag())
        .map(|tag| (tag.title().map(|s| s.to_string()), tag.artist().map(|s| s.to_string())))
        .unwrap_or((None, None));

    // Volume is now set on the Sink rather than baked into the source with
    // amplify(), so it can be changed live and survives seeks correctly.
    let initial_volume: f32 = match call.get_flag_value("amplify") {
        Some(Value::Float { val, .. }) => (val as f32).clamp(0.0, VOLUME_MAX),
        _ => 1.0,
    };

    // Prefer rodio's own duration; fall back to lofty's container-header duration
    // so that minimp3 (which cannot seek-scan) still reports the correct length
    // without needing a manual -d flag.
    let source_duration: Option<Duration> = source.total_duration().or_else(|| {
        tagged_file_res
            .ok()
            .map(|tf| tf.properties().duration())
            .filter(|d| !d.is_zero())
    });

    let sink = Sink::connect_new(output_stream.mixer());
    sink.append(source);
    sink.set_volume(initial_volume);

    let sleep_duration: Duration = match load_duration_from(call, "duration") {
        Some(d) => d,
        None => match source_duration {
            Some(d) => d,
            None => Duration::from_secs(3600),
        },
    };

    let no_progress = call.has_flag("no-progress").unwrap_or(false);

    if no_progress {
        wait_silent(engine, call, &sink, sleep_duration)
    } else {
        let icon_set = resolve_icon_set(call);
        wait_with_progress(engine, call, &sink, sleep_duration, initial_volume, icon_set, title, artist)
    }
}

// ---------------------------------------------------------------------------
// Icon set resolution
// ---------------------------------------------------------------------------

/// Resolves the icon set to use, in priority order:
///   1. `--nerd-fonts` flag
///   2. `NERD_FONTS=1` environment variable
///   3. Unicode if the terminal locale supports UTF-8
///   4. ASCII fallback
fn resolve_icon_set(call: &EvaluatedCall) -> IconSet {
    let flag = call.has_flag("nerd-fonts").unwrap_or(false);
    let env  = std::env::var("NERD_FONTS")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    if flag || env {
        return IconSet::NerdFont;
    }

    if terminal_supports_unicode() {
        IconSet::Unicode
    } else {
        IconSet::Ascii
    }
}

// ---------------------------------------------------------------------------
// Wait strategies
// ---------------------------------------------------------------------------

/// Waits for playback to finish without rendering any output to stderr.
///
/// Exits early when `sink.empty()` returns `true` so the command returns promptly
/// at the real end of the stream rather than sleeping for the full `total` duration.
fn wait_silent(
    engine: &EngineInterface,
    call: &EvaluatedCall,
    sink: &Sink,
    total: Duration,
) -> Result<(), LabeledError> {
    let start = Instant::now();

    while start.elapsed() < total && !sink.empty() {
        engine.signals().check(&call.head)?;
        std::thread::sleep(KEY_POLL_INTERVAL);
    }

    Ok(())
}

/// Renders a live progress line (and optional header) to stderr while the sink plays.
///
/// For files longer than [`CONTROLS_THRESHOLD`] the terminal is placed in raw mode and
/// keyboard events (space, arrows, `m`, `q`) are processed. Raw mode is always restored
/// on exit, even if an error occurs.
fn wait_with_progress(
    engine: &EngineInterface,
    call: &EvaluatedCall,
    sink: &Sink,
    total: Duration,
    initial_volume: f32,
    icons: IconSet,
    title: Option<String>,
    artist: Option<String>,
) -> Result<(), LabeledError> {
    let mut err = stderr();
    let interactive = total >= CONTROLS_THRESHOLD;

    let mut position  = Duration::ZERO;
    let mut last_render = Instant::now().checked_sub(RENDER_INTERVAL).unwrap_or(Instant::now());
    let mut paused    = false;
    let mut volume    = initial_volume;
    let mut pre_mute_volume = initial_volume;
    let mut first_render = true;

    let _ = execute!(err, Hide);

    // Pre-compute the header string once; render_progress will redraw it every frame.
    let header: Option<String> = {
        let parts: Vec<&str> = [artist.as_deref(), title.as_deref()]
            .into_iter()
            .flatten()
            .collect();

        if !parts.is_empty() {
            let header_text = parts.join(" — ");
            let prefix = format!("{}  ", icons.music());
            Some(format!("{}{}", prefix, header_text))
        } else {
            None
        }
    };

    if interactive {
        if let Err(e) = enable_raw_mode() {
            let _ = execute!(err, Show);
            return Err(LabeledError::new(e.to_string()).with_label("failed to enable raw terminal mode", call.head));
        }
    }

    let result = (|| {
        loop {
            // Cap at total: some codecs briefly report a position slightly
            // beyond the stream duration, which would incorrectly trip the
            // end-of-track check or clamp the progress bar to 100% too early.
            position = sink.get_pos().min(total);

            if position >= total || sink.empty() {
                break;
            }

            engine.signals().check(&call.head)?;

            let mut needs_render = false;

            if interactive {
                if event::poll(Duration::ZERO).unwrap_or(false) {
                    if let Ok(Event::Key(KeyEvent { code, kind, .. })) = event::read() {
                        if kind == event::KeyEventKind::Press {
                            match code {
                            // Space — toggle play/pause.
                            KeyCode::Char(' ') => {
                                if paused { sink.play(); paused = false; }
                                else      { sink.pause(); paused = true; }
                                needs_render = true;
                            }
                            // Right / 'l' — seek forward.
                            KeyCode::Right | KeyCode::Char('l') => {
                                let target = (position + SEEK_STEP).min(total);
                                let _ = sink.try_seek(target);
                                needs_render = true;
                            }
                            // Left / 'h' — seek backward.
                            KeyCode::Left | KeyCode::Char('h') => {
                                let target = position.saturating_sub(SEEK_STEP);
                                let _ = sink.try_seek(target);
                                needs_render = true;
                            }
                            // Up / 'k' — volume up.
                            KeyCode::Up | KeyCode::Char('k') => {
                                volume = (volume + VOLUME_STEP).min(VOLUME_MAX);
                                if volume > 0.0 { pre_mute_volume = volume; }
                                sink.set_volume(volume);
                                needs_render = true;
                            }
                            // Down / 'j' — volume down.
                            KeyCode::Down | KeyCode::Char('j') => {
                                volume = (volume - VOLUME_STEP).max(0.0);
                                if volume > 0.0 { pre_mute_volume = volume; }
                                sink.set_volume(volume);
                                needs_render = true;
                            }
                            // 'm' — toggle mute (sets volume to 0 / restores).
                            KeyCode::Char('m') => {
                                if volume > 0.0 {
                                    pre_mute_volume = volume;
                                    volume = 0.0;
                                } else {
                                    volume = pre_mute_volume.max(VOLUME_STEP);
                                }
                                sink.set_volume(volume);
                                needs_render = true;
                            }
                            // 'q' / Escape — stop.
                            KeyCode::Char('q') | KeyCode::Esc => {
                                sink.stop();
                                break;
                            }
                            _ => {}
                            }
                        }
                    }
                }
            }

            if needs_render || last_render.elapsed() >= RENDER_INTERVAL {
                render_progress(&mut err, position, total, paused, volume, interactive, &icons, header.as_deref(), first_render);
                first_render = false;
                last_render = Instant::now();
            }
            std::thread::sleep(KEY_POLL_INTERVAL);
        }

        render_progress(&mut err, position.min(total), total, false, volume, interactive, &icons, header.as_deref(), first_render);
        Ok::<(), LabeledError>(())
    })();

    if interactive {
        let _ = disable_raw_mode();
    }
    if header.is_some() {
        let _ = execute!(err, MoveToColumn(0), Clear(ClearType::CurrentLine));
        let _ = execute!(err, Show, MoveUp(1), MoveToColumn(0), Clear(ClearType::CurrentLine));
    } else {
        let _ = execute!(err, Show, MoveToColumn(0), Clear(ClearType::CurrentLine));
    }

    result
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Minimum terminal width below which the progress line is suppressed to
/// avoid garbled wrapping output.
const MIN_RENDER_WIDTH: u16 = 40;

/// Renders one progress line in-place on stderr.
///
/// Nerd Font:  ♪   0:42 / 4:05  [████████░░░░░░░░░░░░░░░░░░░░░░]  17%   100%  « [SPACE] »  [q]
/// Unicode:    ♪ ▶  0:42 / 4:05  [████████░░░░░░░░░░░░░░░░░░░░░░]  17%  🔊 100%  « [SPACE] »  [q]
/// ASCII:      > 0:42 / 4:05  [########......................]  17%  [V] 100%  << [SPACE] >>  [q]
fn render_progress(
    err: &mut std::io::Stderr,
    elapsed: Duration,
    total: Duration,
    paused: bool,
    volume: f32,
    interactive: bool,
    icons: &IconSet,
    header: Option<&str>,
    first_render: bool,
) {
    // Bail out silently on very narrow terminals rather than wrapping garbage.
    if size().map(|(w, _)| w).unwrap_or(u16::MAX) < MIN_RENDER_WIDTH {
        return;
    }

    let elapsed_str = format_duration(elapsed);
    let total_str   = format_duration(total);
    let ratio = if total.is_zero() {
        0.0
    } else {
        (elapsed.as_secs_f64() / total.as_secs_f64()).clamp(0.0, 1.0)
    };
    let percent     = (ratio * 100.0).round() as u8;
    let vol_pct     = (volume.min(VOLUME_MAX) * 100.0).round() as u8;
    let vol_icon    = icons.volume(volume);

    let prefix = if *icons == IconSet::NerdFont {
        format!("{} ", icons.music())
    } else {
        String::new()
    };
    let icon = if paused { icons.pause() } else { icons.play() };

    let controls_suffix = if interactive {
        let toggle_label = if paused { "play " } else { "pause" };
        format!(
            "  {} [SPACE/{toggle_label}] {}  [↑↓/kj] vol  [m] mute  [q] quit",
            icons.rewind(),
            icons.fast_forward(),
        )
    } else {
        String::new()
    };

    // Dynamic width calculation
    let mut bar_width = 30;
    let mut vol_bar_width = 10;

    if let Ok((cols, _)) = size() {
        let overhead = prefix.width()
            + icon.width()
            + 2 // "  "
            + elapsed_str.width()
            + 3 // " / "
            + total_str.width()
            + 2 // "  "
            + 2 // "[]" main bar
            + 2 // "  "
            + percent.to_string().width()
            + 1 // "%"
            + 2 // "  "
            + vol_icon.width()
            + 1 // " "
            + 2 // "[]" vol bar
            + 1 // " "
            + vol_pct.to_string().width()
            + 1 // "%"
            + controls_suffix.width();

        let available = (cols as usize).saturating_sub(overhead);
        // We want: bar_width + vol_bar_width <= available
        // vol_bar_width = max(5, bar_width / 3)
        // If bar_width >= 15, vol = bar_width / 3. Total = 4/3 * bar_width.
        // If bar_width < 15, vol = 5. Total = bar_width + 5.

        let min_vol = 5usize;
        let max_bar = available.saturating_sub(min_vol);
        let target = (available * 3) / 4;
        bar_width = if target >= 15 {
            target
        } else {
            available.saturating_sub(min_vol)
        };
        bar_width = bar_width.clamp(10, 60).min(max_bar);
        vol_bar_width = (bar_width / 3).max(min_vol);
        // Final guard: ensure bar_width + vol_bar_width never exceeds available.
        if bar_width + vol_bar_width > available {
            bar_width = available.saturating_sub(vol_bar_width).max(10);
        }
    }

    let bar = render_bar(ratio, bar_width, icons);
    let vol_ratio = (volume as f64 / VOLUME_MAX as f64).clamp(0.0, 1.0);
    let vol_bar = render_bar(vol_ratio, vol_bar_width, icons);

    // Build the entire output (header + progress line) into a single buffer so
    // it is written to the terminal in one write_all + flush — eliminating the
    // partial-state flicker that multiple separate write!/queue! calls cause on
    // Windows.
    let mut buf: Vec<u8> = Vec::new();

    if let Some(hdr) = header {
        if first_render {
            // Reserve a blank line that will become the header line.  The
            // cursor ends up one line below it, which is exactly where the
            // progress line lives from this point on.
            let _ = buf.write_all(b"\n");
        }
        // Move up to the header line, clear it, and redraw.
        let _ = queue!(buf, MoveUp(1));
        let _ = queue!(buf, MoveToColumn(0));

        let term_width = size().map(|(w, _)| w).unwrap_or(80) as usize;
        if hdr.width() > term_width {
            let ellipsis = if *icons == IconSet::Ascii { "..." } else { "…" };
            let max_len = term_width.saturating_sub(ellipsis.width());
            let mut width = 0;
            let mut truncated = String::new();
            for c in hdr.chars() {
                let w = c.width().unwrap_or(0);
                if width + w > max_len {
                    break;
                }
                width += w;
                truncated.push(c);
            }
            let _ = write!(buf, "{}{}", truncated, ellipsis);
        } else {
            let _ = buf.write_all(hdr.as_bytes());
        }

        let _ = queue!(buf, Clear(ClearType::UntilNewLine));
        // Drop back down to the progress line.
        let _ = buf.write_all(b"\n");
    }

    // Redraw the progress line.
    let _ = queue!(buf, MoveToColumn(0));
    let _ = queue!(buf, SetAttribute(Attribute::Bold));
    let _ = buf.write_all(format!("{prefix}{icon}").as_bytes());
    let _ = queue!(buf, SetAttribute(Attribute::Reset));
    let _ = buf.write_all(
        format!("  {elapsed_str} / {total_str}  {bar}  {percent}%  {vol_icon} {vol_bar} {vol_pct}%{controls_suffix}")
            .as_bytes(),
    );
    let _ = queue!(buf, Clear(ClearType::UntilNewLine));

    let _ = err.write_all(&buf);
    let _ = err.flush();
}

/// Renders a single progress bar of the given `width` as a `String`.
///
/// For [`IconSet::NerdFont`] a fractional leading block character is used for
/// sub-cell precision; other icon sets round to the nearest whole cell.
fn render_bar(ratio: f64, width: usize, icons: &IconSet) -> String {
    let ratio = ratio.clamp(0.0, 1.0);
    let f_width = ratio * width as f64;

    let n_full = if *icons == IconSet::NerdFont {
        (f_width.floor() as usize).min(width)
    } else {
        (f_width.round() as usize).min(width)
    };

    let bytes_per_char = match icons {
        IconSet::Ascii => 1,
        _ => 3, // NerdFont and Unicode use 3-byte UTF-8 chars (e.g. █ U+2588, ░ U+2591)
    };
    let mut s = String::with_capacity(width * bytes_per_char + 2);
    s.push('[');

    for _ in 0..n_full {
        s.push_str(icons.fill());
    }

    let mut current_len = n_full;

    if current_len < width {
        if *icons == IconSet::NerdFont {
            let remainder = f_width - n_full as f64;
            let part_idx = (remainder * 8.0).floor() as usize;
            if part_idx > 0 {
                let partials = ['▏', '▎', '▍', '▌', '▋', '▊', '▉'];
                if part_idx <= partials.len() {
                    s.push(partials[part_idx - 1]);
                    current_len += 1;
                }
            }
        }
    }

    while current_len < width {
        s.push_str(icons.empty());
        current_len += 1;
    }

    s.push(']');
    s
}

/// Returns `true` if the current terminal environment is likely to support Unicode.
fn terminal_supports_unicode() -> bool {
    #[cfg(target_os = "windows")]
    {
        std::env::var("WT_SESSION").is_ok()
            || std::env::var("ConEmuPID").is_ok()
            || std::env::var("TERM_PROGRAM").map(|v| v == "vscode").unwrap_or(false)
            || std::env::var("ANSICON").is_ok()
    }

    #[cfg(not(target_os = "windows"))]
    {
        let lang = std::env::var("LANG")
            .or_else(|_| std::env::var("LC_ALL"))
            .unwrap_or_default()
            .to_uppercase();
        lang.contains("UTF-8") || lang.contains("UTF8")
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Reads a nushell `Duration` flag by name and converts it to [`std::time::Duration`].
/// Returns `None` if the flag was not supplied or contains a negative value.
fn load_duration_from(call: &EvaluatedCall, name: &str) -> Option<Duration> {
    match call.get_flag_value(name) {
        Some(Value::Duration { val, .. }) if val >= 0 => Some(Duration::from_nanos(val as u64)),
        _ => None,
    }
}
