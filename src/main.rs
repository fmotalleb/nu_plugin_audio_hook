use nu_plugin_audio_hook::Sound;

fn main() {
    let _ = env_logger::try_init();
    nu_plugin::serve_plugin(&mut Sound {}, nu_plugin::MsgPackSerializer {})
}
