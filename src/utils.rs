use nu_plugin::{EngineInterface, EvaluatedCall};
use nu_protocol::{LabeledError, Span, Value};
use std::{fs::File, path::PathBuf, str::FromStr};

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
