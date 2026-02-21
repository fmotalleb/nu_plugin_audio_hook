use nu_plugin_audio_hook::Sound;

fn main() {
    env_logger::init();
    nu_plugin::serve_plugin(&mut Sound {}, nu_plugin::MsgPackSerializer {})
}
