use log::error;
use std::error::Error;
use std::io::{stdin, Write};
use std::path::Path;

pub fn main() {
    init_logger();
    if let Err(err) = try_main() {
        error!("{err}");
        std::process::exit(1);
    }

    stdin().read_line(&mut String::new()).unwrap();
}

#[allow(unused_must_use)]
fn try_main() -> Result<(), Box<dyn Error>> {
    let src_dir_path = Path::new("../../../assets/shader_compilation/original/");
    let target_dir_path = Path::new("../../../assets/shader_compilation/compiled/");

    vku::compile_all_shaders(&src_dir_path, &target_dir_path, true)?;

    Ok(())
}

fn init_logger() {
    let env = env_logger::Env::default()
        .write_style_or("RUST_LOG_STYLE", "always")
        .filter_or("RUST_LOG", "trace");

    env_logger::Builder::from_env(env)
        .target(env_logger::Target::Stderr)
        .format(|buf, record| {
            let mut style = buf.style();

            match record.level() {
                log::Level::Info => style.set_color(env_logger::fmt::Color::Green),
                log::Level::Warn => style.set_color(env_logger::fmt::Color::Yellow),
                log::Level::Error => style.set_color(env_logger::fmt::Color::Red),
                _ => style.set_color(env_logger::fmt::Color::White),
            };

            let timestamp = buf.timestamp();

            writeln!(
                buf,
                "{:<20} : {:<5} : {}",
                timestamp,
                style.value(record.level()),
                record.args()
            )
        })
        .init();
}
