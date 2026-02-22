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
    constants::{get_meta_records, ID3_HASHMAP},
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
            .required_named("key", SyntaxShape::String, "id3 key", Some('k'))
            .required_named("value", SyntaxShape::String, "id3 value", Some('v'))
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "set an ID3 frame on an audio file"
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
    let file_size = file_value.metadata().map(|m| m.len()).unwrap_or(0);

    let tagged_file = read_from_path(&path).ok();
    let mut other = record! {};
    if let Err(e) = file_value.rewind() {
        return Err(LabeledError::new(e.to_string()).with_label("error seeking file", call.head));
    }

    other.push("size", Value::filesize(file_size as i64, call.head));
    if let Some(ext) = path.extension() {
        other.push("format", Value::string(ext.to_string_lossy().to_string(), call.head));
    }

    if let Ok(source) = Decoder::try_from(file_value) {
        if let Some(d) = source.total_duration() {
            let nanos = d.as_nanos().try_into().unwrap_or(0);
            other.push("duration", Value::duration(nanos, call.head));
        } else {
            warn!("Duration unavailable for source");
            other.push("duration", Value::nothing(call.head));
            // TODO: fallback estimation by filesize
        }
        other.push("sample_rate", Value::int(source.sample_rate() as i64, call.head));
        other.push("channels", Value::int(source.channels() as i64, call.head));
    }

    if let Some(tagged_file) = tagged_file {
        if let Some(tag) = tagged_file.primary_tag() {
            for (key, val) in ID3_HASHMAP.iter() {
                if let Some(result) = tag.get_string(val) {
                    insert_into_str(
                        &mut other,
                        key,
                        Some(result.to_string()),
                        call.head,
                    )
                }
            }

            insert_into_integer(&mut other, "track_no", tag.track(), call.head);
            insert_into_integer(&mut other, "total_tracks", tag.track_total(), call.head);
            insert_into_integer(&mut other, "disc_no", tag.disk(), call.head);
            insert_into_integer(&mut other, "total_discs", tag.disk_total(), call.head);
        }
    }

    Ok(Value::record(other, call.head))
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

    let item_key = ID3_HASHMAP.get(key.as_str()).ok_or_else(|| {
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
