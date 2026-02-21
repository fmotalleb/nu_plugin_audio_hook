use chrono::Utc;
use nu_plugin::{self, EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, Span, SyntaxShape, Value};
use rodio::{source::Source, Decoder, OutputStreamBuilder};

use std::{fs::File, path::PathBuf, str::FromStr, time::Duration};

use crate::Sound;

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
                "duration of file (mandatory for non-wave formats like mp3) (default 1 hour)",
                Some('d'),
            )
            .named(
                "amplify",
                SyntaxShape::Float,
                "amplify or attenuate the sound by given value (e.g. 0.5 for half volume)",
                Some('a'),
            )
            .category(Category::Experimental)
    }
    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "play a sound and exits after 5min",
                example: "sound play audio.mp4 -d 5min",
                result: None,
            },
            Example {
                description: "play a sound with 2x volume",
                example: "sound play audio.mp3 -a 2.0",
                result: None,
            },
            Example {
                description: "play a sound with 50% volume",
                example: "sound play audio.mp3 -a 0.5",
                result: None,
            },
            Example {
                description: "play a sound for its duration",
                example: "sound meta audio.mp4 | sound play audio.mp3 -d $in.duration",
                result: None,
            },
        ]
    }
    fn description(&self) -> &str {
        "play an audio file, by default supports flac,Wav,mp3 and ogg files, install plugin with `all-decoders` feature to include aac and mp4(audio)"
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

fn play_audio(engine: &EngineInterface, call: &EvaluatedCall) -> Result<(), LabeledError> {
    let (file_span, file) = load_file(engine, call)?;

    let mut output_stream = match OutputStreamBuilder::open_default_stream() {
        Ok(value) => value,
        Err(err) => {
            return Err(
                LabeledError::new(err.to_string()).with_label("audio stream exception", call.head)
            )
        }
    };

    output_stream.log_on_drop(false);

    let source = match Decoder::try_from(file) {
        Ok(value) => value,
        Err(err) => {
            return Err(
                LabeledError::new(err.to_string()).with_label("audio decoder exception", file_span)
            )
        }
    };

    let amplify: f32 = match call.get_flag_value("amplify") {
        Some(Value::Float { val, .. }) => val as f32,
        _ => 1.0,
    };

    let duration = source.total_duration();
    output_stream.mixer().add(source.amplify(amplify));

    let sleep_duration: Duration = match load_duration_from(call, "duration") {
        Some(duration) => duration,
        None => match duration {
            Some(duration) => duration,
            None => Duration::from_secs(3600),
        },
    };

    let sleep_until = Utc::now() + sleep_duration;

    // We check for OS signals
    while engine.signals().check(&call.head).map(|_| true)? && Utc::now() < sleep_until {
        // We yield to the OS until necessary
        std::thread::yield_now();
    }

    Ok(())
}

fn load_duration_from(call: &EvaluatedCall, name: &str) -> Option<Duration> {
    match call.get_flag_value(name) {
        Some(Value::Duration { val, .. }) => {
            Some(Duration::from_nanos(u64::from_ne_bytes(val.to_ne_bytes())))
        }
        _ => None,
    }
}

fn load_file(engine: &EngineInterface, call: &EvaluatedCall) -> Result<(Span, File), LabeledError> {
    let file_path: Value = call.req(0).map_err(|e| {
        LabeledError::new(e.to_string()).with_label("Expected file path", call.head)
    })?;

    let span = file_path.span();

    let file_path = match file_path {
        Value::String { val, .. } => PathBuf::from_str(&val)
            .map_err(|e| LabeledError::new(e.to_string()).with_label("Invalid path format", span)),
        _ => Err(LabeledError::new("invalid input").with_label("Expected file path", span)),
    }?;

    let file_path = resolve_filepath(engine, span, file_path)?;

    let file_handle = File::open(file_path).map_err(|e| {
        LabeledError::new(e.to_string()).with_label("error trying to open the file", span)
    })?;

    Ok((span, file_handle))
}

fn resolve_filepath(
    engine: &EngineInterface,
    span: Span,
    file_path: PathBuf,
) -> Result<PathBuf, LabeledError> {
    let file_path = if file_path.is_absolute() {
        Ok::<PathBuf, LabeledError>(file_path)
    } else {
        let current_path = engine.get_current_dir().map_err(|e| {
            LabeledError::new(e.to_string()).with_label("Could not get current directory", span)
        })?;
        let base = PathBuf::from_str(current_path.as_str()).map_err(|e| {
            LabeledError::new(e.to_string()).with_label(
                "Could not convert path provided by engine to PathBuf object (issue in nushell)",
                span,
            )
        })?;
        Ok(base.join(file_path))
    }?
    .canonicalize()
    .map_err(|e| LabeledError::new(e.to_string()).with_label("File not found", span))?;
    Ok(file_path)
}
