use nu_plugin::{EvaluatedCall, SimplePluginCommand};
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
            .switch(
                "data",
                "output binary data (WAV) instead of playing",
                Some('d'),
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
            Example {
                description: "save a noise to a file",
                example: "sound make 1000 200ms --data | save output.wav",
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
    let (frequency_value, duration_value, amplify_value) = load_values(call)?;

    if call
        .has_flag("data")
        .map_err(|e| LabeledError::new(e.to_string()))?
    {
        let wav_data = generate_wav(frequency_value, duration_value, amplify_value)?;
        Ok(Value::binary(wav_data, call.head))
    } else {
        sine_wave(frequency_value, duration_value, amplify_value)?;
        Ok(Value::nothing(call.head))
    }
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

fn generate_wav(
    frequency: f32,
    duration: Duration,
    amplify: f32,
) -> Result<Vec<u8>, LabeledError> {
    let source = SineWave::new(frequency)
        .take_duration(duration)
        .amplify(amplify);
    let sample_rate = 48000u32;
    let samples: Vec<i16> = source
        .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect();

    let num_channels = 1u16;
    let bits_per_sample = 16u16;
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let subchunk2_size = samples.len() as u32 * num_channels as u32 * bits_per_sample as u32 / 8;
    let chunk_size = 36 + subchunk2_size;

    let mut buffer = Vec::with_capacity(44 + subchunk2_size as usize);

    // RIFF header
    buffer.extend_from_slice(b"RIFF");
    buffer.extend_from_slice(&chunk_size.to_le_bytes());
    buffer.extend_from_slice(b"WAVE");

    // fmt subchunk
    buffer.extend_from_slice(b"fmt ");
    buffer.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size for PCM
    buffer.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat (1 = PCM)
    buffer.extend_from_slice(&num_channels.to_le_bytes());
    buffer.extend_from_slice(&sample_rate.to_le_bytes());
    buffer.extend_from_slice(&byte_rate.to_le_bytes());
    buffer.extend_from_slice(&block_align.to_le_bytes());
    buffer.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data subchunk
    buffer.extend_from_slice(b"data");
    buffer.extend_from_slice(&subchunk2_size.to_le_bytes());

    for sample in samples {
        buffer.extend_from_slice(&sample.to_le_bytes());
    }

    Ok(buffer)
}

fn load_values(call: &EvaluatedCall) -> Result<(f32, Duration, f32), LabeledError> {
    let frequency: Value = call.req(0).map_err(|err| {
        LabeledError::new(err.to_string()).with_label("Frequency value not found", call.head)
    })?;

    let frequency_value: f32 = match frequency.as_float() {
        Ok(value) => value as f32,
        Err(err) => {
            return Err(LabeledError::new(err.to_string()).with_label(
                "Frequency value must be of type Float (f32)",
                frequency.span(),
            ))
        }
    };
    let duration: Value = call.req(1).map_err(|err| {
        LabeledError::new(err.to_string()).with_label("Duration value not found", call.head)
    })?;

    let duration_value = match duration {
        Value::Duration { val, .. } => Duration::from_nanos(val.try_into().unwrap_or(0)),
        _ => {
            return Err(LabeledError::new("cannot parse duration value as Duration")
                .with_label("Expected duration", duration.span()))
        }
    };

    let amplify: Value = match call.get_flag("amplify") {
        Ok(value) => match value {
            Some(value) => value,
            None => Value::float(1.0, call.head),
        },
        Err(err) => {
            return Err(LabeledError::new(err.to_string())
                .with_label("Amplify value error", call.head))
        }
    };
    let amplify_value: f32 = amplify.as_float().map_err(|err| {
        LabeledError::new(err.to_string())
            .with_label("Amplify value must be of type Float (f32)", amplify.span())
    })? as f32;
    Ok((frequency_value, duration_value, amplify_value))
}
