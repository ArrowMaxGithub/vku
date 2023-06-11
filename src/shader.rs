#![cfg(feature = "shader")]

use std::fs::{create_dir_all, read_dir, read_to_string, remove_dir_all};

use crate::imports::*;
use shaderc::CompilationArtifact;

/// Compiles all GLSL shaders in ```src_dir_path``` to SPIR-V shader binaries in ```target_dir_path``` alongside optional debug text results.
///
/// The shader kind is read from the shader's file extension:
/// - .frag => Fragment shader
/// - .vert => Vertex shader
/// - .comp => Compute shader
/// - .glsl => Include source for other shaders
///
/// .glsl files may be used in other shaders as copy-paste include directives, but they have to provide a relative path from the calling .exe to the include file:
///
/// ```#include "./assets/shaders/src/example.glsl"```
///
/// Only a single entry point main() is allowed.
#[allow(unused_must_use)]

pub fn compile_all_shaders(
    src_dir_path: &Path,
    target_dir_path: &Path,
    debug: bool,
) -> Result<(), Error> {
    trace!("Compiling all shaders inside {src_dir_path:?} to {target_dir_path:?}");
    remove_dir_all(target_dir_path);
    create_dir_all(target_dir_path)?;

    let compiler = shaderc::Compiler::new().unwrap();

    let mut compiler_options = shaderc::CompileOptions::new().unwrap();
    if debug {
        compiler_options.set_optimization_level(shaderc::OptimizationLevel::Zero);
        compiler_options.set_generate_debug_info();
    } else {
        compiler_options.set_optimization_level(shaderc::OptimizationLevel::Performance);
    }

    compiler_options.set_include_callback(shader_include_callback);

    let shaders_dir = read_dir(src_dir_path)?;
    for entry in shaders_dir {
        let shader_entry = entry?;
        let path = shader_entry.path();
        let extension = path.extension();
        let file_type_string = extension.unwrap().to_str().unwrap();
        if file_type_string == "glsl" {
            continue;
        }
        let shader_kind = match file_type_string {
            "frag" => Ok(shaderc::ShaderKind::Fragment),
            "vert" => Ok(shaderc::ShaderKind::Vertex),
            "comp" => Ok(shaderc::ShaderKind::Compute),
            _ => Err(Error::UnknownShaderFileExtension),
        }?;

        let shader_src = read_to_string(&path)?;
        let shader_name = path
            .file_name()
            .unwrap()
            .to_ascii_lowercase()
            .into_string()
            .unwrap();
        let shader_ext = path
            .extension()
            .unwrap()
            .to_ascii_lowercase()
            .into_string()
            .unwrap();

        compile_shader(
            shader_src,
            &shader_name,
            &shader_ext,
            target_dir_path,
            &compiler,
            shader_kind,
            Some(&compiler_options),
            debug,
        )?;
    }

    Ok(())
}

/// Compile single shader module from String without writing to a file.
#[allow(unused_must_use)]

pub fn shader_ad_hoc(
    shader_src: String,
    shader_name: &str,
    shader_ext: &str,
    debug: bool,
) -> Result<CompilationArtifact, Error> {
    let compiler = shaderc::Compiler::new().unwrap();

    let mut compiler_options = shaderc::CompileOptions::new().unwrap();
    if debug {
        compiler_options.set_optimization_level(shaderc::OptimizationLevel::Zero);
        compiler_options.set_generate_debug_info();
    } else {
        compiler_options.set_optimization_level(shaderc::OptimizationLevel::Performance);
    }

    compiler_options.set_include_callback(shader_include_callback);

    let shader_kind = match shader_ext {
        "frag" => Ok(shaderc::ShaderKind::Fragment),
        "vert" => Ok(shaderc::ShaderKind::Vertex),
        "comp" => Ok(shaderc::ShaderKind::Compute),
        _ => Err(Error::UnknownShaderFileExtension),
    }?;

    compile_shader_adhoc(
        shader_src,
        shader_name,
        &compiler,
        shader_kind,
        Some(&compiler_options),
    )
}

fn compile_shader_adhoc(
    shader_src: String,
    shader_name: &str,
    compiler: &shaderc::Compiler,
    kind: shaderc::ShaderKind,
    add_options: Option<&shaderc::CompileOptions>,
) -> Result<CompilationArtifact, Error> {
    trace!("Compiling shader {shader_name:?}");

    let preprocess = compiler.preprocess(&shader_src, shader_name, "main", add_options)?;

    let binary_result = match compiler.compile_into_spirv(
        &preprocess.as_text(),
        kind,
        shader_name,
        "main",
        add_options,
    ) {
        Ok(result) => result,
        Err(e) => {
            let preprocess_text = preprocess.as_text();
            let preprocess_numbered: Vec<String> = preprocess_text
                .split('\n')
                .enumerate()
                .map(|(index, line)| format!("{}: {line}", index + 1))
                .collect();
            for p in preprocess_numbered {
                println!("{p}");
            }

            return Err(Error::Preprocess(e));
        }
    };

    Ok(binary_result)
}

fn compile_shader(
    shader_src: String,
    shader_name: &str,
    shader_ext: &str,
    target_path: &Path,
    compiler: &shaderc::Compiler,
    kind: shaderc::ShaderKind,
    add_options: Option<&shaderc::CompileOptions>,
    debug: bool,
) -> Result<(), Error> {
    trace!("Compiling shader {shader_name:?}");

    let preprocess = compiler.preprocess(&shader_src, shader_name, "main", add_options)?;

    let binary_result = compiler.compile_into_spirv(
        &preprocess.as_text(),
        kind,
        shader_name,
        "main",
        add_options,
    )?;
    let binary_extension = String::from(shader_ext) + ".spv";
    let binary_path = Path::new(target_path)
        .join(shader_name)
        .with_extension(binary_extension);

    if debug {
        let text_result = compiler.compile_into_spirv_assembly(
            &preprocess.as_text(),
            kind,
            shader_name,
            "main",
            add_options,
        )?;
        let mut text_extension = String::from(shader_ext);
        text_extension.push_str(".txt");
        let text_path = Path::new(target_path)
            .join(shader_name)
            .with_extension(text_extension);

        std::fs::write(text_path, text_result.as_text())?;
    }

    std::fs::write(binary_path, binary_result.as_binary_u8())?;

    Ok(())
}

fn shader_include_callback(
    src_path: &str,
    _include_type: shaderc::IncludeType,
    dst_name: &str,
    _include_depth: usize,
) -> shaderc::IncludeCallbackResult {
    trace!("Including file {src_path:?} for shader {dst_name:?}");

    let res_content = read_to_string(src_path).unwrap();

    let res_include = shaderc::ResolvedInclude {
        resolved_name: src_path.to_string(),
        content: res_content,
    };

    shaderc::IncludeCallbackResult::Ok(res_include)
}
