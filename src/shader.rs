use crate::{errors::ShaderCompilationError, imports::*};
use shaderc::CompilationArtifact;
use spirv_reflect::{create_shader_module, types::*};

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

#[allow(unused_must_use)]
pub fn shader_ad_hoc(
    shader_src: String,
    shader_name: &str,
    shader_ext: &str,
    debug: bool,
) -> Result<CompilationArtifact> {
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
        _ => Err(ShaderCompilationError::UnknownShaderFileExtension),
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
) -> Result<CompilationArtifact> {
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

            return Err(anyhow!(
                "Shader compilation failed, see preprocess trace above. {e}"
            ));
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
) -> Result<()> {
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

#[derive(Debug)]
pub struct ReflectionResult {
    pub shader_stage: ShaderStageFlags,
    pub input_attributes: Vec<VertexInputAttributeDescription>,
    pub desc_set_layout_infos: Vec<DescriptorSetLayoutCreateInfo>,
    pub desc_sets_bindings: Vec<Vec<DescriptorSetLayoutBinding>>,
}

///Reflects on provided SPIR-V data and returns [ReflectionResult](crate::ReflectionResult).
///
/// Unsupported features:
/// - Specialization constants
///
/// Gotchas:
/// - Shader input interface reports the expected type and does not know about any compression or normalization.
///
/// e.g. Vec4 inside shader => R32G32B32A32_SFLOAT as interface, but the actual data provided by the application is R8G8B8A8_UNORM.
pub fn reflect_spirv_shader(spv_data: &[u8]) -> Result<ReflectionResult> {
    match create_shader_module(spv_data) {
        Ok(ref mut module) => {
            let shader_stage = match module.get_shader_stage() {
                ReflectShaderStageFlags::VERTEX => ShaderStageFlags::VERTEX,
                ReflectShaderStageFlags::TESSELLATION_CONTROL => {
                    ShaderStageFlags::TESSELLATION_CONTROL
                }
                ReflectShaderStageFlags::TESSELLATION_EVALUATION => {
                    ShaderStageFlags::TESSELLATION_EVALUATION
                }
                ReflectShaderStageFlags::GEOMETRY => ShaderStageFlags::GEOMETRY,
                ReflectShaderStageFlags::FRAGMENT => ShaderStageFlags::FRAGMENT,
                ReflectShaderStageFlags::COMPUTE => ShaderStageFlags::COMPUTE,
                ReflectShaderStageFlags::RAYGEN_BIT_NV => ShaderStageFlags::RAYGEN_KHR,
                ReflectShaderStageFlags::ANY_HIT_BIT_NV => ShaderStageFlags::ANY_HIT_KHR,
                ReflectShaderStageFlags::CLOSEST_HIT_BIT_NV => ShaderStageFlags::CLOSEST_HIT_KHR,
                ReflectShaderStageFlags::MISS_BIT_NV => ShaderStageFlags::MISS_KHR,
                ReflectShaderStageFlags::INTERSECTION_BIT_NV => ShaderStageFlags::INTERSECTION_KHR,
                ReflectShaderStageFlags::CALLABLE_BIT_NV => ShaderStageFlags::CALLABLE_KHR,
                ReflectShaderStageFlags::UNDEFINED => ShaderStageFlags::ALL,
                _ => ShaderStageFlags::ALL,
            };
            trace!("{shader_stage:?}");

            let mut input_vars_reflect = module.enumerate_input_variables(None).unwrap();
            input_vars_reflect.sort_by(|a, b| a.location.partial_cmp(&b.location).unwrap());

            let mut vertex_input_attributes = Vec::new();
            let mut offset = 0;
            for (location, input_var) in input_vars_reflect.into_iter().enumerate() {
                trace!("{input_var:?}");
                let (format, size) = match input_var.format {
                    ReflectFormat::Undefined => (Format::UNDEFINED, 0),
                    ReflectFormat::R32_UINT => (Format::R32_UINT, 4),
                    ReflectFormat::R32_SINT => (Format::R32_SINT, 4),
                    ReflectFormat::R32_SFLOAT => (Format::R32_SFLOAT, 4),
                    ReflectFormat::R32G32_UINT => (Format::R32G32_UINT, 8),
                    ReflectFormat::R32G32_SINT => (Format::R32G32_SINT, 8),
                    ReflectFormat::R32G32_SFLOAT => (Format::R32G32_SFLOAT, 8),
                    ReflectFormat::R32G32B32_UINT => (Format::R32G32B32_UINT, 12),
                    ReflectFormat::R32G32B32_SINT => (Format::R32G32B32_SINT, 12),
                    ReflectFormat::R32G32B32_SFLOAT => (Format::R32G32B32_SFLOAT, 12),
                    ReflectFormat::R32G32B32A32_UINT => (Format::R32G32B32A32_UINT, 16),
                    ReflectFormat::R32G32B32A32_SINT => (Format::R32G32B32A32_SINT, 16),
                    ReflectFormat::R32G32B32A32_SFLOAT => (Format::R32G32B32A32_SFLOAT, 16),
                };
                let vertex_input_attribute = VertexInputAttributeDescription::builder()
                    .location(location as u32)
                    .offset(offset)
                    .format(format)
                    .binding(0)
                    .build();
                trace!("{vertex_input_attribute:?}");

                vertex_input_attributes.push(vertex_input_attribute);
                offset += size;
            }

            let mut desc_set_layout_infos = Vec::new();
            let mut desc_sets_bindings = Vec::new();

            let sets_reflect = module.enumerate_descriptor_sets(None).unwrap();
            for set in sets_reflect {
                info!("{set:?}");
                let descriptor_set_layout_bindings: Vec<DescriptorSetLayoutBinding> = set
                    .bindings
                    .iter()
                    .map(|b| {
                        let ty = match b.descriptor_type {
                            ReflectDescriptorType::Undefined => DescriptorType::SAMPLER,
                            ReflectDescriptorType::Sampler => DescriptorType::SAMPLER,
                            ReflectDescriptorType::CombinedImageSampler => {
                                DescriptorType::COMBINED_IMAGE_SAMPLER
                            }
                            ReflectDescriptorType::SampledImage => DescriptorType::SAMPLED_IMAGE,
                            ReflectDescriptorType::StorageImage => DescriptorType::STORAGE_IMAGE,
                            ReflectDescriptorType::UniformTexelBuffer => {
                                DescriptorType::UNIFORM_TEXEL_BUFFER
                            }
                            ReflectDescriptorType::StorageTexelBuffer => {
                                DescriptorType::STORAGE_TEXEL_BUFFER
                            }
                            ReflectDescriptorType::UniformBuffer => DescriptorType::UNIFORM_BUFFER,
                            ReflectDescriptorType::StorageBuffer => DescriptorType::STORAGE_BUFFER,
                            ReflectDescriptorType::UniformBufferDynamic => {
                                DescriptorType::UNIFORM_BUFFER_DYNAMIC
                            }
                            ReflectDescriptorType::StorageBufferDynamic => {
                                DescriptorType::STORAGE_BUFFER_DYNAMIC
                            }
                            ReflectDescriptorType::InputAttachment => {
                                DescriptorType::INPUT_ATTACHMENT
                            }
                            ReflectDescriptorType::AccelerationStructureNV => {
                                DescriptorType::ACCELERATION_STRUCTURE_KHR
                            }
                        };
                        DescriptorSetLayoutBinding::builder()
                            .binding(b.binding)
                            .descriptor_type(ty)
                            .descriptor_count(1)
                            .stage_flags(shader_stage)
                            .build()
                    })
                    .collect();

                let descriptor_set_layout = DescriptorSetLayoutCreateInfo::builder()
                    .bindings(&descriptor_set_layout_bindings)
                    .build();

                desc_set_layout_infos.push(descriptor_set_layout);
                desc_sets_bindings.push(descriptor_set_layout_bindings);
            }

            Ok(ReflectionResult {
                shader_stage,
                input_attributes: vertex_input_attributes,
                desc_set_layout_infos,
                desc_sets_bindings,
            })
        }
        Err(err) => Err(anyhow!(err)),
    }
}
