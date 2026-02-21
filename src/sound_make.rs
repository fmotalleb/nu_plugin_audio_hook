use nu_plugin::{self, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, Span, SyntaxShape, Value};
use rodio::source::{SineWave, Source};
use rodio::{OutputStreamBuilder, Sink};

use std::time::Duration;

use crate::Sound;

pub struct SoundMakeCmd;

impl SimplePluginCommand for SoundMakeCmd {
    type Plugin = Sound;

    fn name(&self) -> &str {
        "sound make"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::new("sound make")
            .required("Frequency", SyntaxShape::Float, "Frequency of the noise")
            .required("duration", SyntaxShape::Duration, "duration of the noise")
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
                description: "create a simple noise frequency",
                example: "sound make 1000 200ms",
                result: None,
            },
            Example {
                description: "create a simple noise frequency with 50% volume",
                example: "sound make 1000 200ms -a 0.5",
                result: None,
            },
            Example {
                description: "create a simple noise sequence",
                example:
                    "[ 300.0, 500.0,  1000.0, 400.0, 600.0 ] | each { |it| sound make $it 150ms }",
                result: None,
            },
        ]
    }
    fn description(&self) -> &str {
        "creates a noise with given frequency and duration"
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &nu_plugin::EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, nu_protocol::LabeledError> {
        make_sound(call)
    }
}

pub struct SoundBeepCmd;

impl SimplePluginCommand for SoundBeepCmd {
    type Plugin = Sound;

    fn name(&self) -> &str {
        "sound beep"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::new("sound beep").category(Category::Experimental)
    }
    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "create a simple beep sound",
            example: "sound beep",
            result: None,
        }]
    }
    fn description(&self) -> &str {
        "creates a beep noise"
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &nu_plugin::EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, nu_protocol::LabeledError> {
        sine_wave(1000.0, Duration::from_millis(300), 1.0)?;
        return Ok(Value::nothing(call.head));
    }
}

fn make_sound(call: &EvaluatedCall) -> Result<Value, LabeledError> {
    let (frequency_value, duration_value, amplify_value) = match load_values(call) {
        Ok(value) => value,
        Err(value) => return value,
    };

    sine_wave(frequency_value, duration_value, amplify_value)?;
    Ok(Value::nothing(call.head))
}

fn sine_wave(
    frequency_value: f32,
    duration_value: Duration,
    amplify_value: f32,
) -> Result<(), LabeledError> {
    let mut stream_handle = OutputStreamBuilder::open_default_stream().map_err(|err| {
        LabeledError::new(err.to_string()).with_label("audio stream exception", Span::unknown())
    })?;

    stream_handle.log_on_drop(false);

    let sink = Sink::connect_new(stream_handle.mixer());
    let source = SineWave::new(frequency_value)
        .take_duration(duration_value)
        .amplify(amplify_value);
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

fn load_values(call: &EvaluatedCall) -> Result<(f32, Duration, f32), Result<Value, LabeledError>> {
    let frequency: Value = match call.req(0) {
        Ok(value) => value,
        Err(err) => {
            return Err(Err(LabeledError::new(err.to_string())
                .with_label("Frequency value not found", call.head)))
        }
    };
    let frequency_value: f32 = match frequency.as_float() {
        Ok(value) => value as f32,
        Err(err) => {
            return Err(Err(LabeledError::new(err.to_string()).with_label(
                "Frequency value must be of type Float (f32)",
                frequency.span(),
            )))
        }
    };
    let duration: Value = match call.req(1) {
        Ok(value) => value,
        Err(err) => {
            return Err(Err(LabeledError::new(err.to_string())
                .with_label("Duration value not found", call.head)))
        }
    };
    let duration_value = match duration {
        Value::Duration { val, .. } => Duration::from_nanos(val.try_into().unwrap_or(0)),
        _ => {
            return Err(Err(LabeledError::new(
                "cannot parse duration value as Duration",
            )))
        }
    };

    let amplify: Value = match call.get_flag("amplify") {
        Ok(value) => match value {
            Some(value) => value,
            None => Value::float(1.0, Span::unknown()),
        },
        Err(err) => {
            return Err(Err(LabeledError::new(err.to_string())
                .with_label("Duration value not found", call.head)))
        }
    };
    let amplify_value: f32 = match amplify.as_float() {
        Ok(value) => value as f32,
        Err(err) => {
            return Err(Err(LabeledError::new(err.to_string()).with_label(
                "Amplify value must be of type Float (f32)",
                amplify.span(),
            )))
        }
    };
    Ok((frequency_value, duration_value, amplify_value))
}
