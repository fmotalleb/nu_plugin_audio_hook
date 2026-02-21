use std::collections::HashMap;

use lazy_static::lazy_static;
use nu_protocol::{record, Span, Value};

lazy_static! {
    pub static ref ID3_HASHMAP: HashMap<&'static str, &'static str> = HashMap::from([
        ("album", "TALB"),
        ("albumartist", "TPE2"),
        ("albumsortorder", "TSOA"),
        ("arranger", "TPE4"),
        ("artist", "TPE1"),
        ("audiodelay", "TDLY"),
        ("audiolength", "TLEN"),
        ("audiosize", "TSIZ"),
        ("author", "TOLY"),
        ("bpm", "TBPM"),
        ("composer", "TCOM"),
        ("conductor", "TPE3"),
        ("copyright", "TCOP"),
        ("date", "TDAT"),
        ("discnumber", "TPOS"),
        ("encodedby", "TENC"),
        ("encodingsettings", "TSSE"),
        ("filename", "TOFN"),
        ("fileowner", "TOWN"),
        ("filetype", "TFLT"),
        ("genre", "TCON"),
        ("grouping", "TIT1"),
        ("initialkey", "TKEY"),
        ("isrc", "TSRC"),
        ("itunesalbumsortorder", "TSO2"),
        ("itunescompilationflag", "TCMP"),
        ("itunescomposersortorder", "TSOC"),
        ("language", "TLAN"),
        ("lyricist", "TEXT"),
        ("mediatype", "TMED"),
        ("mood", "TMOO"),
        ("organization", "TPUB"),
        ("originalalbum", "TOAL"),
        ("originalartist", "TOPE"),
        ("originalyear", "TORY"),
        ("performersortorder", "TSOP"),
        ("producednotice", "TPRO"),
        ("radioowner", "TRSO"),
        ("radiostationname", "TRSN"),
        ("recordingdates", "TRDA"),
        ("setsubtitle", "TSST"),
        ("time", "TIME"),
        ("title", "TIT2"),
        ("titlesortorder", "TSOT"),
        ("track", "TRCK"),
        ("version", "TIT3"),
        ("year", "TYER"),
        ("wwwartist", "WOAR"),
        ("wwwcommercialinfo", "WCOM"),
        ("wwwcopyright", "WCOP"),
        ("wwwfileinfo", "WOAF"),
        ("wwwpayment", "WPAY"),
        ("wwwpublisher", "WPUB"),
        ("wwwradio", "WORS"),
        ("wwwsource", "WOAS"),
        ("encodingtime", "TDEN"),
        ("originalreleasetime", "TDOR"),
        ("releasetime", "TDRL"),
        ("taggingtime", "TDTG"),
        ("recordingyear", "TDRC")
    ]);
}
pub fn get_meta_records(span: Span) -> Value {
    let mut result: Vec<Value> = vec![];
    for (key, val) in ID3_HASHMAP.iter() {
        result.push(Value::record(
            record! {
                "normalized"=>Value::string(key.to_string(), span),
                "frame_name"=>Value::string(val.to_string(), span),
            },
            span,
        ));
    }
    return Value::list(result, span);
}
