use crate::{errors::ShaderCompilationError, imports::*};
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

        compile_shader(
            &shader_entry,
            target_dir_path,
            &compiler,
            shader_kind,
            Some(&compiler_options),
            debug,
        )?;
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

        std::fs::write(text_path, text_result.as_text())?;
    }

    std::fs::write(binary_path, binary_result.as_binary_u8())?;

    Ok(())
}

#[cfg(test)]
pub(crate) fn compile_shader_from_text(
    name: &str,
    raw_code: &str,
    kind: shaderc::ShaderKind,
) -> Result<(String, Vec<u8>)> {
    trace!("Compiling shader {name:?}");
    let compiler = shaderc::Compiler::new().unwrap();

    let mut compiler_options = shaderc::CompileOptions::new().unwrap();
    compiler_options.set_optimization_level(shaderc::OptimizationLevel::Zero);
    compiler_options.set_generate_debug_info();

    let text_result = compiler
        .compile_into_spirv_assembly(raw_code, kind, name, "main", Some(&compiler_options))?
        .as_text();

    let binary_result = compiler
        .compile_into_spirv(raw_code, kind, name, "main", Some(&compiler_options))?
        .as_binary_u8()
        .to_owned();

    Ok((text_result, binary_result))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflect_vertex_shader() {
        let code = r#"
        #version 450
        layout(push_constant) uniform Push {
            mat4 matrix;
            vec4 data0;
            vec4 data1;
            vec4 data2;
            vec4 data3;
        } push;

        layout(location = 0) in vec2 i_pos;
        layout(location = 1) in vec2 i_uv;
        layout(location = 2) in vec4 i_col;

        layout(location = 0) out vec4 o_col;
        layout(location = 1) out vec2 o_uv;

        vec3 srgb_to_linear(vec3 srgb) {
            bvec3 cutoff = lessThan(srgb, vec3(0.04045));
            vec3 lower = srgb / vec3(12.92);
            vec3 higher = pow((srgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
            return mix(higher, lower, cutoff);
        }

        void main() {
            o_uv = i_uv;
            o_col = vec4(srgb_to_linear(i_col.rgb), i_col.a);
            gl_Position = push.matrix * vec4(i_pos.x, i_pos.y, 0.0, 1.0);
        }"#;

        let (_, binary) =
            compile_shader_from_text("vertex_shader_test", code, shaderc::ShaderKind::Vertex)
                .unwrap();
        let reflect_result = reflect_spirv_shader(&binary).unwrap();

        assert_eq!(reflect_result.shader_stage, ShaderStageFlags::VERTEX);

        let expected_vertex_attributes = [
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(Format::R32G32_SFLOAT)
                .offset(0)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(Format::R32G32_SFLOAT)
                .offset(8)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(Format::R32G32B32A32_SFLOAT)
                .offset(16)
                .build(),
        ];

        assert_eq!(
            reflect_result.input_attributes.len(),
            expected_vertex_attributes.len()
        );
        for i in 0..reflect_result.input_attributes.len() {
            assert_eq!(
                reflect_result.input_attributes[i].location,
                expected_vertex_attributes[i].location
            );
            assert_eq!(
                reflect_result.input_attributes[i].binding,
                expected_vertex_attributes[i].binding
            );
            assert_eq!(
                reflect_result.input_attributes[i].format,
                expected_vertex_attributes[i].format
            );
            assert_eq!(
                reflect_result.input_attributes[i].offset,
                expected_vertex_attributes[i].offset
            );
        }

        assert_eq!(reflect_result.desc_sets_bindings.len(), 0);
        assert_eq!(reflect_result.desc_set_layout_infos.len(), 0);
    }

    #[test]
    fn reflect_fragment_shader() {
        let code = r#"
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
        }"#;

        let (_, binary) =
            compile_shader_from_text("fragment_shader_test", code, shaderc::ShaderKind::Fragment)
                .unwrap();

        let reflect_result = reflect_spirv_shader(&binary).unwrap();

        assert_eq!(reflect_result.shader_stage, ShaderStageFlags::FRAGMENT);

        let expected_set_layout_bindings = [DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(ShaderStageFlags::FRAGMENT)
            .build()];

        assert_eq!(reflect_result.desc_sets_bindings.len(), 1);
        for binding in &reflect_result.desc_sets_bindings[0] {
            assert_eq!(binding.binding, expected_set_layout_bindings[0].binding);
            assert_eq!(
                binding.descriptor_type,
                expected_set_layout_bindings[0].descriptor_type
            );
            assert_eq!(
                binding.descriptor_count,
                expected_set_layout_bindings[0].descriptor_count
            );
            assert_eq!(
                binding.stage_flags,
                expected_set_layout_bindings[0].stage_flags
            );
        }

        let expected_set_layout_create_infos = [DescriptorSetLayoutCreateInfo::builder()
            .bindings(&expected_set_layout_bindings)
            .build()];

        assert_eq!(reflect_result.desc_set_layout_infos.len(), 1);
        assert_eq!(
            reflect_result.desc_set_layout_infos[0].binding_count,
            expected_set_layout_create_infos[0].binding_count
        );
    }

    #[test]
    fn reflect_compute_shader() {
        let code = r#"
        #version 460
        struct Particle{
            float pos_x;
            float pos_y;
            float vel_x;
            float vel_y;
            float acc_x;
            float acc_y;
        
            float size;
            float life;
            float mass;
            float color;
        };

        struct ParticleVertex{
            float pos_x;
            float pos_y;
            float size;
            float life;
            float color;
        };

        const uint MAX_PARTICLES_COUNT = 1024 * 1024 * 16;
        const uint MAX_PARTICLES_PER_FRAME = 1024 * 1024;

        const float DELTA_TIME = 0.01;

        //device-local buffer
        layout (set = 0, binding = 0) restrict buffer VertexBuffer{
            ParticleVertex data[MAX_PARTICLES_COUNT];
        } vertex_buffer;

        //device-local buffer
        layout (set = 0, binding = 1) restrict buffer ParticleBuffer{
            Particle data[MAX_PARTICLES_COUNT];
        } particle_buffer;

        //Dispatch on MAX_PARTICLES_COUNT, 1, 1
        void main(){
            uint index = gl_GlobalInvocationID.x;

            Particle particle = particle_buffer.data[index];
            ParticleVertex vertex = vertex_buffer.data[index];

            particle.vel_x += DELTA_TIME * particle.acc_x;
            particle.vel_y += DELTA_TIME * particle.acc_y;

            particle.pos_x += DELTA_TIME * particle.vel_x;
            particle.pos_y += DELTA_TIME * particle.vel_y;

            vertex.pos_x = particle.pos_x;
            vertex.pos_y = particle.pos_y;
            vertex.size = particle.size;
            vertex.life = particle.life;
            vertex.color = particle.color;

            particle_buffer.data[index] = particle;
            vertex_buffer.data[index] = vertex;
        }"#;

        let (_, binary) =
            compile_shader_from_text("compute_shader_test", code, shaderc::ShaderKind::Compute)
                .unwrap();

        let reflect_result = reflect_spirv_shader(&binary).unwrap();

        assert_eq!(reflect_result.shader_stage, ShaderStageFlags::COMPUTE);

        let expected_set_layout_bindings = [
            DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(ShaderStageFlags::COMPUTE)
                .build(),
            DescriptorSetLayoutBinding::builder()
                .binding(1)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(ShaderStageFlags::COMPUTE)
                .build(),
        ];

        assert_eq!(reflect_result.desc_sets_bindings.len(), 1);
        assert_eq!(reflect_result.desc_sets_bindings[0].len(), 2);

        for i in 0..2 {
            assert_eq!(
                reflect_result.desc_sets_bindings[0][i].binding,
                expected_set_layout_bindings[i].binding
            );
            assert_eq!(
                reflect_result.desc_sets_bindings[0][i].descriptor_type,
                expected_set_layout_bindings[i].descriptor_type
            );
            assert_eq!(
                reflect_result.desc_sets_bindings[0][i].descriptor_count,
                expected_set_layout_bindings[i].descriptor_count
            );
            assert_eq!(
                reflect_result.desc_sets_bindings[0][i].stage_flags,
                expected_set_layout_bindings[i].stage_flags
            );
        }

        let expected_set_layout_create_infos = [DescriptorSetLayoutCreateInfo::builder()
            .bindings(&expected_set_layout_bindings)
            .build()];

        assert_eq!(reflect_result.desc_set_layout_infos.len(), 1);
        assert_eq!(
            reflect_result.desc_set_layout_infos[0].binding_count,
            expected_set_layout_create_infos[0].binding_count
        );
    }
}
