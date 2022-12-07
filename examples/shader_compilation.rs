use anyhow::Result;
use log::error;
use std::io::{Write, stdin};
use std::path::Path;

pub fn main() {
    init_logger();
    if let Err(err) = try_main() {
        err.chain().for_each(|cause| error!("{}", cause));
        std::process::exit(1);
    }

    stdin().read_line(&mut String::new()).unwrap();
}

#[allow(unused_must_use)]
fn try_main() -> Result<()>{
    let src_dir_path = Path::new("./assets/shaders/src/");
    let target_dir_path = Path::new("./assets/shaders/compiled_shaders/");

    let src_glsl_path = src_dir_path.join(Path::new("example.glsl"));
    let src_vert_path = src_dir_path.join(Path::new("example.vert"));

    //Remove previous runs if necessary
    std::fs::remove_dir_all(&src_dir_path);
    std::fs::remove_dir_all(&target_dir_path);

    std::fs::create_dir_all(&src_dir_path)?;
    std::fs::create_dir_all(&target_dir_path)?;

    std::fs::write(&src_glsl_path, r#"
    struct Example{
        float pos_x;
        float pos_y;
        float pos_z;
        float size;
    
        float color;
    };"#)?;

    std::fs::write(&src_vert_path, r#"
    #version 450
    #include "./assets/shaders/src/example.glsl" //path relative to the .exe calling VkInit::compile_all_shaders
    
    layout(location = 0) in vec4 i_pos_size;
    layout(location = 1) in vec4 i_col;
    
    layout(location = 0) out vec4 o_col;
    
    void main() {
        o_col = i_col;
        gl_Position = vec4(i_pos_size.xyz, 1.0);
        gl_PointSize  = i_pos_size.w;
    }"#)?;

    vku::shader::compile_all_shaders(&src_dir_path, &target_dir_path, true)?;

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