use anyhow::Result;
use log::error;
use std::io::{stdin, Write};
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
fn try_main() -> Result<()> {
    let src_dir_path = Path::new("./assets/shaders/src/");
    let target_dir_path = Path::new("./assets/shaders/compiled_shaders/");

    let src_egui_fragment_shader_path = src_dir_path.join(Path::new("egui_fragment.frag"));
    let compiled_egui_fragment_shader_path =
        target_dir_path.join(Path::new("egui_fragment.frag.spv"));

    //Remove previous runs if necessary
    std::fs::remove_dir_all(&src_dir_path);
    std::fs::remove_dir_all(&target_dir_path);

    std::fs::create_dir_all(&src_dir_path)?;
    std::fs::create_dir_all(&target_dir_path)?;

    std::fs::write(
        &src_egui_fragment_shader_path,
        r#"
    #version 450

    layout(location = 0) in vec4 o_color;
    layout(location = 1) in vec2 o_uv;

    layout(binding = 0, set = 0) uniform sampler2D fonts_sampler;

    layout(location = 0) out vec4 final_color;

    vec3 srgb_gamma_from_linear(vec3 rgb) {
        bvec3 cutoff = lessThan(rgb, vec3(0.0031308));
        vec3 lower = rgb * vec3(12.92);
        vec3 higher = vec3(1.055) * pow(rgb, vec3(1.0 / 2.4)) - vec3(0.055);
        return mix(higher, lower, vec3(cutoff));
    }

    vec4 srgba_gamma_from_linear(vec4 rgba) {
        return vec4(srgb_gamma_from_linear(rgba.rgb), rgba.a);
    }

    void main() {
    #if SRGB_TEXTURES
        vec4 tex = srgba_gamma_from_linear(texture(fonts_sampler, o_uv));
    #else
        vec4 tex = texture(fonts_sampler, o_uv);
    #endif

        final_color = o_color * tex;
    }"#,
    )?;

    vku::shader::compile_all_shaders(&src_dir_path, &target_dir_path, true)?;
    let spv_data = std::fs::read(&compiled_egui_fragment_shader_path)?;
    let _ = vku::reflect_spirv_shader(&spv_data)?;
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
