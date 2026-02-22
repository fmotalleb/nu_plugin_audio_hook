use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::prelude::Accessor;
use lofty::{read_from_path, tag::Tag};
use log::warn;
use nu_plugin::{EvaluatedCall, SimplePluginCommand};
use nu_protocol::{record, Category, LabeledError, Record, Signature, Span, SyntaxShape, Type, Value};
use rodio::{Decoder, Source};
use std::io::Seek;
use std::time::Duration;
use std::collections::HashSet;

use crate::{
    constants::{get_meta_records, TAG_MAP},
    utils::{format_duration, load_file},
    Sound,
};
/// Nushell command `sound meta set` — writes a single metadata tag to an audio file.
///
/// Accepts a file path, a format-agnostic key name (`-k`), and a string value (`-v`).
/// The key is looked up in [`TAG_MAP`] (case-insensitive) and written via lofty so the
/// same key name works across MP3, FLAC, OGG, and MP4.
pub struct SoundMetaSetCmd;
impl SimplePluginCommand for SoundMetaSetCmd {
    type Plugin = Sound;

    fn name(&self) -> &str {
        "sound meta set"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::new("sound meta set")
            .required("File Path", SyntaxShape::Filepath, "file to update")
            .required_named("key", SyntaxShape::String, "metadata key", Some('k'))
            .required_named("value", SyntaxShape::String, "metadata value", Some('v'))
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "set a metadata tag on an audio file"
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &nu_plugin::EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, nu_protocol::LabeledError> {
        audio_meta_set(engine, call)
    }
}

/// Nushell command `sound meta` — reads metadata and file properties from an audio file.
///
/// With `--all` prints the full [`TAG_MAP`] key reference instead of reading a file.
/// Otherwise returns a record containing file size, format, container bitrate,
/// tag fields, numeric track/disc info, embedded artwork, and decoded-stream properties
/// (duration, sample rate, channels).
pub struct SoundMetaGetCmd;
impl SimplePluginCommand for SoundMetaGetCmd {
    type Plugin = Sound;

    fn name(&self) -> &str {
        "sound meta"
    }

    fn signature(&self) -> Signature {
        Signature::new("sound meta")
            .input_output_types(vec![
                (Type::Nothing, Type::Record(vec![].into())),
                (Type::Binary,  Type::Record(vec![].into())),
            ])
            .switch("all", "List all possible frame names", Some('a'))
            .optional("File Path", SyntaxShape::Filepath, "file to play")
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "get duration and metadata of an audio file"
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &nu_plugin::EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, nu_protocol::LabeledError> {
        if let Value::Binary { .. } = input {
            return Err(LabeledError::new(
                "binary pipeline input is not yet supported — streaming support is planned",
            )
            .with_label("unsupported input", call.head));
        }
        if let Ok(true) = call.has_flag("all") {
            return Ok(get_meta_records(call.head));
        }
        let (_, file, path) = load_file(engine, call)?;
        parse_meta(call, file, path)
    }
}

/// Combines lofty tag data ([`parse_tags`]) with rodio stream data ([`parse_stream_meta`])
/// into a single nushell `Record` value.
fn parse_meta(
    call: &EvaluatedCall,
    mut file_value: std::fs::File,
    path: std::path::PathBuf,
) -> Result<Value, LabeledError> {
    let (mut record, lofty_duration) = parse_tags(&path, call.head)?;

    if let Err(e) = file_value.rewind() {
        return Err(LabeledError::new(e.to_string()).with_label("error seeking file", call.head));
    }

    match Decoder::try_from(file_value) {
        Ok(source) => {
            let stream_meta = parse_stream_meta(&source, lofty_duration, call.head);
            for (col, val) in stream_meta {
                record.push(col, val);
            }
        }
        Err(e) => warn!("Failed to decode audio stream: {}", e),
    }

    Ok(Value::record(record, call.head))
}

/// Reads lofty metadata from `path` and populates a nushell [`Record`].
///
/// Covers file size, format extension, [`FileProperties`] (bitrate, bit depth),
/// all [`TAG_MAP`] text fields, numeric track/disc accessors, and embedded artwork.
/// Opens its own file handle via `std::fs::metadata` / `lofty::read_from_path` so no
/// caller-owned handle is required.
///
/// Returns the record alongside the container-reported duration (if any) so the caller
/// can pass it to [`parse_stream_meta`] as a fallback when rodio cannot determine the
/// duration itself (e.g. with the minimp3 decoder).
fn parse_tags(path: &std::path::Path, span: Span) -> Result<(Record, Option<Duration>), LabeledError> {
    let mut record = record! {};
    let mut lofty_duration: Option<Duration> = None;

    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    record.push("size", Value::filesize(file_size as i64, span));

    if let Some(ext) = path.extension() {
        record.push("format", Value::string(ext.to_string_lossy().to_string(), span));
    }

    let tagged_file_res = read_from_path(path);
    if let Err(ref e) = tagged_file_res {
        warn!("Error reading tags from {:?}: {}", path, e);
    }
    let tagged_file = tagged_file_res.ok();
    if let Some(tagged_file) = tagged_file {
        // ── FileProperties ────────────────────────────────────────────────────
        let props = tagged_file.properties();
        if let Some(v) = props.overall_bitrate() {
            record.push("bitrate", Value::int(v as i64, span));
        }
        if let Some(v) = props.audio_bitrate() {
            record.push("audio_bitrate", Value::int(v as i64, span));
        }
        if let Some(v) = props.bit_depth() {
            record.push("bit_depth", Value::int(v as i64, span));
        }
        // Capture the container-header duration as a fallback for decoders
        // (e.g. minimp3) that cannot determine duration from the stream alone.
        let d = props.duration();
        if !d.is_zero() {
            lofty_duration = Some(d);
        }

        // ── Tag fields ────────────────────────────────────────────────────────
        if let Some(tag) = tagged_file.primary_tag() {
            let mut seen_keys = HashSet::new();
            for (key, val) in TAG_MAP.iter() {
                if *val == lofty::tag::ItemKey::TrackNumber || *val == lofty::tag::ItemKey::DiscNumber {
                    continue;
                }
                if seen_keys.contains(val) {
                    continue;
                }
                if let Some(result) = tag.get_string(*val) {
                    insert_into_str(&mut record, key, Some(result.to_string()), span);
                    seen_keys.insert(*val);
                }
            }

            insert_into_integer(&mut record, "track_no", tag.track(), span);
            insert_into_integer(&mut record, "total_tracks", tag.track_total(), span);
            insert_into_integer(&mut record, "disc_no", tag.disk(), span);
            insert_into_integer(&mut record, "total_discs", tag.disk_total(), span);

            // ── Embedded artwork ──────────────────────────────────────────────
            let pictures = tag.pictures();
            if !pictures.is_empty() {
                let artwork: Vec<Value> = pictures
                    .iter()
                    .map(|pic| {
                        let mut art = record! {
                            "pic_type" => Value::string(format!("{:?}", pic.pic_type()), span),
                            "mime_type" => Value::string(
                                pic.mime_type()
                                    .map(|m| m.as_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                                span,
                            ),
                            "size" => Value::filesize(pic.data().len() as i64, span),
                        };
                        if let Some(desc) = pic.description() {
                            art.push("description", Value::string(desc.to_string(), span));
                        }
                        Value::record(art, span)
                    })
                    .collect();
                record.push("artwork", Value::list(artwork, span));
            }
        }
    }
    Ok((record, lofty_duration))
}

/// Extracts duration, sample rate, and channel count from a rodio [`Source`] and returns
/// them as a nushell [`Record`].
///
/// `lofty_duration` is the container-header duration extracted by [`parse_tags`] and
/// serves as a fallback when `source.total_duration()` returns `None` (e.g. when the
/// minimp3 decoder is in use).  Only emits `Value::nothing` for the duration field when
/// both sources are unavailable.
///
/// Duration strings are formatted via [`format_duration`] (`M:SS` / `H:MM:SS`) so the
/// output matches the live progress display in `audio_player`.
fn parse_stream_meta(source: &impl Source, lofty_duration: Option<Duration>, span: Span) -> Record {
    let mut record = record! {};
    let duration = source.total_duration().or(lofty_duration);
    if let Some(d) = duration {
        record.push("duration", Value::string(format_duration(d), span));
    } else {
        warn!("Duration unavailable for source");
        record.push("duration", Value::nothing(span));
        // TODO: fallback estimation by filesize
    }
    record.push("sample_rate", Value::int(source.sample_rate() as i64, span));
    record.push("channels", Value::int(source.channels() as i64, span));
    record
}

/// Core implementation of `sound meta set`.
///
/// Looks up the normalised key in [`TAG_MAP`], obtains or creates the primary tag,
/// calls `insert_text`, saves the file in-place, then re-reads and returns the
/// updated metadata record so the caller always sees the final on-disk state.
fn audio_meta_set(engine: &nu_plugin::EngineInterface, call: &EvaluatedCall) -> Result<Value, LabeledError> {
    let (_, file_value, path) = load_file(engine, call)?;
    let key = match call.get_flag_value("key") {
        Some(Value::String { val, .. }) => val,
        _ => {
            return Err(LabeledError::new("set key using `-k` flag".to_string())
                .with_label("cannot get value of key", call.head));
        }
    };
    let value = match call.get_flag_value("value") {
        Some(Value::String { val, .. }) => val,
        _ => {
            return Err(LabeledError::new("set value using `-v` flag".to_string())
                .with_label("cannot get value of value", call.head));
        }
    };
    drop(file_value);

    let mut tagged_file = read_from_path(&path).map_err(|e| {
        LabeledError::new(e.to_string()).with_label("error reading file", call.head)
    })?;

    let normalized_key = key.to_lowercase();
    let item_key = TAG_MAP.get(normalized_key.as_str()).cloned().ok_or_else(|| {
        LabeledError::new(format!("Unknown metadata key: {}", normalized_key))
            .with_label("key not found", call.head)
    })?;

    let tag = match tagged_file.primary_tag_mut() {
        Some(tag) => tag,
        None => {
            let tag_type = tagged_file.file_type().primary_tag_type();
            tagged_file.insert_tag(Tag::new(tag_type));
            tagged_file.primary_tag_mut().ok_or_else(|| {
                LabeledError::new("failed to create primary tag for file".to_string())
                    .with_label("tag insertion failed", call.head)
            })?
        }
    };

    let tag_type = tag.tag_type();
    if !tag.insert_text(item_key, value) {
        return Err(LabeledError::new(format!(
            "tag type {:?} rejected key '{}'",
            tag_type, normalized_key
        ))
        .with_label("insert_text returned false", call.head));
    }

    tagged_file.save_to_path(&path, WriteOptions::default()).map_err(|e| {
        LabeledError::new(e.to_string()).with_label("error saving file", call.head)
    })?;

    let file = std::fs::File::open(&path).map_err(|e| {
        LabeledError::new(e.to_string()).with_label("error re-opening file for parsing", call.head)
    })?;
    parse_meta(call, file, path)
}
/// Pushes a string field into `record` only when `val` is `Some`.
fn insert_into_str(
    record: &mut Record,
    name: impl AsRef<str>,
    val: Option<impl AsRef<str>>,
    span: Span,
) {
    if let Some(val) = val {
        record.push(name.as_ref(), Value::string(val.as_ref(), span));
    }
}

/// Pushes a `u32` field into `record` as a nushell `int` only when `val` is `Some`.
fn insert_into_integer(record: &mut Record, name: &str, val: Option<u32>, span: Span) {
    if let Some(val) = val {
        record.push(name, Value::int(val.into(), span));
    }
}
