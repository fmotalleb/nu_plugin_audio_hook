use id3::{Tag, TagLike};
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

    let tags = match Tag::read_from2(&mut file_value) {
        Ok(tags) => Some(tags),
        Err(_) => None,
    };
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

    match tags {
        Some(tags) => {
            for (key, val) in ID3_HASHMAP.iter() {
                if let Some(result) = tags.get(val) {
                    insert_into_str(
                        &mut other,
                        key,
                        Some(result.content().to_string()),
                        call.head,
                    )
                }
            }

            insert_into_integer(&mut other, "track_no", tags.track(), call.head);
            insert_into_integer(&mut other, "total_tracks", tags.total_tracks(), call.head);
            insert_into_integer(&mut other, "disc_no", tags.disc(), call.head);
            insert_into_integer(&mut other, "total_discs", tags.total_discs(), call.head);
        }
        None => {}
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
    let tags = match Tag::read_from2(&file_value) {
        Ok(tags) => Some(tags),
        Err(_) => None,
    };

    drop(file_value);

    if let Some(mut tags) = tags {
        tags.set_text(key, value);

        let tr = tags.write_to_path(&path, tags.version());
        tr.map_err(|e| {
            LabeledError::new(e.to_string()).with_label("error during writing", call.head)
        })?
    }

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
    match val {
        Some(val) => record.push(name.as_ref(), Value::string(val.as_ref(), span)),
        None => {}
    }
}

fn insert_into_integer(record: &mut Record, name: &str, val: Option<u32>, span: Span) {
    match val {
        Some(val) => record.push(name, Value::int(val.into(), span)),
        None => {}
    }
}
