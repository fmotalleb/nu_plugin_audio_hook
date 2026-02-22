use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::prelude::Accessor;
use lofty::{read_from_path, tag::Tag};
use log::warn;
use nu_plugin::{EvaluatedCall, SimplePluginCommand};
use nu_protocol::{record, Category, LabeledError, Record, Signature, Span, SyntaxShape, Value};
use rodio::{Decoder, Source};
use std::io::Seek;

use crate::{
    constants::{get_meta_records, TAG_MAP},
    utils::load_file,
    Sound,
};
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

pub struct SoundMetaGetCmd;
impl SimplePluginCommand for SoundMetaGetCmd {
    type Plugin = Sound;

    fn name(&self) -> &str {
        "sound meta"
    }

    fn signature(&self) -> Signature {
        Signature::new("sound meta")
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
        _input: &Value,
    ) -> Result<Value, nu_protocol::LabeledError> {
        if let Ok(true) = call.has_flag("all") {
            return Ok(get_meta_records(call.head));
        }
        let (_, file, path) = load_file(engine, call)?;
        parse_meta(call, file, path)
    }
}

fn parse_meta(
    call: &EvaluatedCall,
    mut file_value: std::fs::File,
    path: std::path::PathBuf,
) -> Result<Value, LabeledError> {
    let mut record = parse_tags(&path, call.head)?;

    if let Err(e) = file_value.rewind() {
        return Err(LabeledError::new(e.to_string()).with_label("error seeking file", call.head));
    }

    if let Ok(source) = Decoder::try_from(file_value) {
        let stream_meta = parse_stream_meta(&source, call.head);
        for (col, val) in stream_meta {
            record.push(col, val);
        }
    }

    Ok(Value::record(record, call.head))
}

fn parse_tags(path: &std::path::Path, span: Span) -> Result<Record, LabeledError> {
    let mut record = record! {};

    let file = std::fs::File::open(path).map_err(|e| {
        LabeledError::new(e.to_string()).with_label("error opening file", span)
    })?;
    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
    record.push("size", Value::filesize(file_size as i64, span));

    if let Some(ext) = path.extension() {
        record.push("format", Value::string(ext.to_string_lossy().to_string(), span));
    }

    let tagged_file = read_from_path(path).ok();
    if let Some(tagged_file) = tagged_file {
        if let Some(tag) = tagged_file.primary_tag() {
            for (key, val) in TAG_MAP.iter() {
                if let Some(result) = tag.get_string(val) {
                    insert_into_str(&mut record, key, Some(result.to_string()), span)
                }
            }

            insert_into_integer(&mut record, "track_no", tag.track(), span);
            insert_into_integer(&mut record, "total_tracks", tag.track_total(), span);
            insert_into_integer(&mut record, "disc_no", tag.disk(), span);
            insert_into_integer(&mut record, "total_discs", tag.disk_total(), span);
        }
    }
    Ok(record)
}

fn parse_stream_meta(source: &impl Source, span: Span) -> Record {
    let mut record = record! {};
    if let Some(d) = source.total_duration() {
        let nanos = d.as_nanos().try_into().unwrap_or(0);
        record.push("duration", Value::duration(nanos, span));
    } else {
        warn!("Duration unavailable for source");
        record.push("duration", Value::nothing(span));
        // TODO: fallback estimation by filesize
    }
    record.push("sample_rate", Value::int(source.sample_rate() as i64, span));
    record.push("channels", Value::int(source.channels() as i64, span));
    record
}

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

    let item_key = TAG_MAP.get(key.as_str()).ok_or_else(|| {
        LabeledError::new(format!("Unknown metadata key: {}", key))
            .with_label("key not found", call.head)
    })?;

    let tag = match tagged_file.primary_tag_mut() {
        Some(tag) => tag,
        None => {
            let tag_type = tagged_file.file_type().primary_tag_type();
            tagged_file.insert_tag(Tag::new(tag_type));
            tagged_file.primary_tag_mut().expect("Just inserted tag")
        }
    };

    tag.insert_text(item_key.clone(), value);

    tagged_file.save_to_path(&path, WriteOptions::default()).map_err(|e| {
        LabeledError::new(e.to_string()).with_label("error saving file", call.head)
    })?;

    let file = std::fs::File::open(&path).map_err(|e| {
        LabeledError::new(e.to_string()).with_label("error re-opening file for parsing", call.head)
    })?;
    parse_meta(call, file, path)
}
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

fn insert_into_integer(record: &mut Record, name: &str, val: Option<u32>, span: Span) {
    if let Some(val) = val {
        record.push(name, Value::int(val.into(), span));
    }
}
