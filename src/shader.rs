use crate::{imports::*, errors::ShaderCompilationError};
use spirv_reflect::{ShaderModule, types::*};

/// Compiles all GLSL shaders in ```src_dir_path``` to SPIR-V shaders in ```target_dir_path``` alongside optional debug text results.
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
pub fn compile_all_shaders(src_dir_path: &Path, target_dir_path: &Path, debug: bool) -> Result<()> {
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
            _ => Err(ShaderCompilationError::UnknownShaderFileExtension),
        }?;

        compile_shader(&shader_entry, target_dir_path, &compiler, shader_kind, Some(&compiler_options), debug)?;
    }

    Ok(())
}    

fn compile_shader(
    entry: &DirEntry,
    target_path: &Path,
    compiler: &shaderc::Compiler,
    kind: shaderc::ShaderKind,
    add_options: Option<&shaderc::CompileOptions>,
    debug: bool,
) -> Result<()> {
    let path = entry.path();
    let source = read_to_string(&path)?;
    let file_name = path.file_name().unwrap();
    let name = file_name.to_str().unwrap();
    let extension = path.extension().unwrap().to_str().unwrap();
    trace!("Compiling shader {name:?}");
    
    let preprocess = compiler.preprocess(&source, name, "main", add_options)?;
    
    let binary_result =
        compiler.compile_into_spirv(&preprocess.as_text(), kind, name, "main", add_options)?;
    let binary_extension = String::from(extension) + ".spv";
    let binary_path = Path::new(target_path)
        .join(name)
        .with_extension(binary_extension);
    
    if debug {
        trace!("Compiling shader to text");

        let text_result = compiler.compile_into_spirv_assembly(
            &preprocess.as_text(),
            kind,
            name,
            "main",
            add_options,
        )?;
        let mut text_extension = String::from(extension);
        text_extension.push_str(".txt");
        let text_path = Path::new(target_path)
        .join(name)
        .with_extension(text_extension);

        std::fs::write(&text_path, text_result.as_text())?;
    }
    
    trace!("Compiling shader to SPIR-V");
    std::fs::write(&binary_path, binary_result.as_binary_u8())?;

    Ok(())
}

fn shader_include_callback(
    src_path: &str,
    _include_type: shaderc::IncludeType,
    dst_name: &str,
    _include_depth: usize,
) -> shaderc::IncludeCallbackResult {
    trace!("Including file {src_path:?} for shader {dst_name:?}");

    let res_content = read_to_string(&src_path).unwrap();

    let res_include = shaderc::ResolvedInclude {
        resolved_name: src_path.to_string(),
        content: res_content,
    };

    shaderc::IncludeCallbackResult::Ok(res_include)
}

pub struct ReflectionResult{
    pub shader_stage: ShaderStageFlags,
    pub input_vars: Vec<VertexInputAttributeDescription>,
}

pub fn reflect_spirv_shader(path: &Path) -> Result<()>{
    trace!("Reclecting on shader at {path:?}");
    let mut file = std::fs::File::open(path)?;
    let spv_data = read_spv(&mut file)?;

    match ShaderModule::load_u32_data(&spv_data){
        Ok(ref mut module) => {
            let shader_stage = module.get_shader_stage();
            let input_vars = module.enumerate_input_variables(None).unwrap();
            let output_vars = module.enumerate_output_variables(None).unwrap();
            let bindings = module.enumerate_descriptor_bindings(None).unwrap();
            let sets = module.enumerate_descriptor_sets(None).unwrap();
        }
        Err(err) => return Err(anyhow!(err))
    }

    Ok(())
}