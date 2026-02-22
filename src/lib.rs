//! `nu_plugin_audio_hook` — a Nushell plugin for generating, playing, and
//! inspecting audio files.
//!
//! Registers five commands: `sound beep`, `sound make`, `sound play`,
//! `sound meta`, and `sound meta set`.
mod audio_meta;
mod audio_player;
mod constants;
mod sound;
mod sound_make;
mod utils;
pub use sound::Sound;
// pub use sound_make::make_sound;
