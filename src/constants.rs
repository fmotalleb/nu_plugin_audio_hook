use std::collections::BTreeMap;
use std::sync::LazyLock;

use lofty::tag::ItemKey;
use nu_protocol::{record, Span, Value};

pub static TAG_MAP: LazyLock<BTreeMap<&'static str, ItemKey>> = LazyLock::new(|| {
    BTreeMap::from([
        // Core identity
        ("album",              ItemKey::AlbumTitle),
        ("albumartist",        ItemKey::AlbumArtist),
        ("albumsortorder",     ItemKey::AlbumTitleSortOrder),
        ("artist",             ItemKey::TrackArtist),
        ("artistsortorder",    ItemKey::TrackArtistSortOrder),
        ("title",              ItemKey::TrackTitle),
        ("titlesortorder",     ItemKey::TrackTitleSortOrder),
        ("subtitle",           ItemKey::TrackSubtitle),
        ("setsubtitle",        ItemKey::SetSubtitle),

        // People
        ("composer",           ItemKey::Composer),
        ("composersortorder",  ItemKey::ComposerSortOrder),
        ("conductor",          ItemKey::Conductor),
        ("label",              ItemKey::Label),
        ("lyricist",           ItemKey::Lyricist),
        ("movement",           ItemKey::Movement),
        ("movementnumber",     ItemKey::MovementNumber),
        ("movementtotal",      ItemKey::MovementTotal),
        ("organization",       ItemKey::Publisher),
        ("producer",           ItemKey::Producer),
        ("remixer",            ItemKey::Remixer),
        ("work",               ItemKey::Work),

        // Dates
        ("date",               ItemKey::RecordingDate),
        ("originalyear",       ItemKey::OriginalReleaseDate),
        ("releasedate",        ItemKey::ReleaseDate),
        ("year",               ItemKey::Year),

        // Identifiers
        ("barcode",            ItemKey::Barcode),
        ("cataloguenumber",    ItemKey::CatalogNumber),
        ("isrc",               ItemKey::Isrc),

        // Style & content
        ("bpm",                ItemKey::Bpm),
        ("comment",            ItemKey::Comment),
        ("compilation",        ItemKey::FlagCompilation),
        ("copyright",          ItemKey::CopyrightMessage),
        ("encodedby",          ItemKey::EncodedBy),
        ("encodingsettings",   ItemKey::EncoderSettings),
        ("genre",              ItemKey::Genre),
        ("grouping",           ItemKey::ContentGroup),
        ("initialkey",         ItemKey::InitialKey),
        ("language",           ItemKey::Language),
        ("lyrics",             ItemKey::Lyrics),
        ("mood",               ItemKey::Mood),
        ("originalalbum",      ItemKey::OriginalAlbumTitle),
        ("originalartist",     ItemKey::OriginalArtist),
        ("script",             ItemKey::Script),
        ("track",              ItemKey::TrackNumber),
        ("discnumber",         ItemKey::DiscNumber),

        // ReplayGain
        ("replaygain_album_gain",  ItemKey::ReplayGainAlbumGain),
        ("replaygain_album_peak",  ItemKey::ReplayGainAlbumPeak),
        ("replaygain_track_gain",  ItemKey::ReplayGainTrackGain),
        ("replaygain_track_peak",  ItemKey::ReplayGainTrackPeak),
    ])
});

pub fn get_meta_records(span: Span) -> Value {
    let mut result: Vec<Value> = vec![];
    for (key, val) in TAG_MAP.iter() {
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
