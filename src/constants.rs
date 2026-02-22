use std::collections::HashMap;
use std::sync::LazyLock;

use lofty::tag::ItemKey;
use nu_protocol::{record, Span, Value};

pub static ID3_HASHMAP: LazyLock<HashMap<&'static str, ItemKey>> = LazyLock::new(|| {
    HashMap::from([
        ("album", ItemKey::AlbumTitle),
        ("albumartist", ItemKey::AlbumArtist),
        ("albumsortorder", ItemKey::AlbumTitleSortOrder),
        ("artist", ItemKey::TrackArtist),
        ("bpm", ItemKey::Bpm),
        ("composer", ItemKey::Composer),
        ("conductor", ItemKey::Conductor),
        ("copyright", ItemKey::CopyrightMessage),
        ("date", ItemKey::RecordingDate),
        ("discnumber", ItemKey::DiscNumber),
        ("encodedby", ItemKey::EncodedBy),
        ("encodingsettings", ItemKey::EncoderSettings),
        ("genre", ItemKey::Genre),
        ("grouping", ItemKey::ContentGroup),
        ("initialkey", ItemKey::InitialKey),
        ("isrc", ItemKey::Isrc),
        ("language", ItemKey::Language),
        ("lyricist", ItemKey::Lyricist),
        ("mood", ItemKey::Mood),
        ("organization", ItemKey::Publisher),
        ("originalalbum", ItemKey::OriginalAlbumTitle),
        ("originalartist", ItemKey::OriginalArtist),
        ("originalyear", ItemKey::OriginalReleaseDate),
        ("setsubtitle", ItemKey::SetSubtitle),
        ("title", ItemKey::TrackTitle),
        ("titlesortorder", ItemKey::TrackTitleSortOrder),
        ("track", ItemKey::TrackNumber),
        ("year", ItemKey::Year),
    ])
});

pub fn get_meta_records(span: Span) -> Value {
    let mut result: Vec<Value> = vec![];
    for (key, val) in ID3_HASHMAP.iter() {
        result.push(Value::record(
            record! {
                "normalized"=>Value::string(key.to_string(), span),
                "frame_name"=>Value::string(format!("{:?}", val), span),
            },
            span,
        ));
    }
    return Value::list(result, span);
}
