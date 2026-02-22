use nu_plugin::{EngineInterface, EvaluatedCall};
use nu_protocol::{LabeledError, Span, Value};
use std::{fs::File, path::PathBuf, time::Duration};

pub fn resolve_filepath(
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
        Ok(PathBuf::from(current_path).join(file_path))
    }?
    .canonicalize()
    .map_err(|e| LabeledError::new(e.to_string()).with_label("Failed to canonicalize path", span))?;
    Ok(file_path)
}

pub fn load_file_path(
    engine: &EngineInterface,
    call: &EvaluatedCall,
) -> Result<(Span, PathBuf), LabeledError> {
    let file_path: Value = call.req(0).map_err(|e| {
        LabeledError::new(e.to_string()).with_label("Expected file path", call.head)
    })?;

    let span = file_path.span();

    let file_path = match file_path {
        Value::String { val, .. } => PathBuf::from(val),
        _ => return Err(LabeledError::new("invalid input").with_label("Expected file path", span)),
    };

    let file_path = resolve_filepath(engine, span, file_path)?;
    Ok((span, file_path))
}

/// Formats a [`Duration`] as `M:SS`, or `H:MM:SS` for durations ≥ 1 hour.
///
/// Shared by `audio_meta` (duration field in metadata records) and `audio_player`
/// (live progress display) so both always produce identical output.
pub fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours   = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

pub fn load_file(
    engine: &EngineInterface,
    call: &EvaluatedCall,
) -> Result<(Span, File, PathBuf), LabeledError> {
    let (span, path) = load_file_path(engine, call)?;
    let file = File::open(&path).map_err(|e| {
        LabeledError::new(e.to_string()).with_label("error trying to open the file", span)
    })?;
    Ok((span, file, path))
}
