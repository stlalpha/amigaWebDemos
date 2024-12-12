use wasm_bindgen::prelude::*;
use web_sys::{WebGlRenderingContext, WebGlProgram, WebGlShader, WebGlTexture, WebGlBuffer};
use fontdue::Font;

const FONT_DATA: &[u8] = include_bytes!("topaza1200.ttf");

#[wasm_bindgen]
pub struct DemoEffect {
    context: WebGlRenderingContext,
    copper_program: WebGlProgram,
    effect_program: WebGlProgram,
    text_texture: WebGlTexture,
    quad_buffer: WebGlBuffer,
    time: f32,
    resolution: (i32, i32),
    text_scale: f32,
}

#[wasm_bindgen]
impl DemoEffect {
    pub fn new(canvas_id: &str) -> Result<DemoEffect, JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id(canvas_id).unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;
        
        let context = canvas.get_context("webgl")?.unwrap().dyn_into::<WebGlRenderingContext>()?;
        let canvas_width = canvas.width() as i32;
        let canvas_height = canvas.height() as i32;

        // Copper shader setup
        let copper_vertex_shader = compile_shader(
            &context,
            WebGlRenderingContext::VERTEX_SHADER,
            r#"
                attribute vec4 position;
                void main() {
                    gl_Position = position;
                }
            "#,
        )?;

        let copper_fragment_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
                precision mediump float;
                uniform float time;
                
                void main() {
                    float y = gl_FragCoord.y;
                    float wave = sin(y * 0.1 + time) * 0.5 + 0.5;
                    vec3 color = vec3(wave * 0.2, wave * 0.8, wave * 0.7);
                    gl_FragColor = vec4(color, 1.0);
                }
            "#,
        )?;

        let copper_program = link_program(&context, &copper_vertex_shader, &copper_fragment_shader)?;

        // Effect shader setup
        let effect_vertex_shader = compile_shader(
            &context,
            WebGlRenderingContext::VERTEX_SHADER,
            r#"
                attribute vec4 position;
                varying vec2 fragCoord;
                
                void main() {
                    fragCoord = position.xy * 0.5 + 0.5;
                    gl_Position = position;
                }
            "#,
        )?;

        let effect_fragment_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
                precision highp float;
                varying vec2 fragCoord;
                uniform sampler2D iChannel0;
                uniform float iTime;
                
                vec3 copperBars(vec2 uv) {
                    float y = uv.y;
                    float wave = sin(y * 20.0 + iTime * 2.0) * 0.5 + 0.5;
                    return vec3(wave * 0.2, wave * 0.8, wave * 0.7);
                }
                
                void main() {
                    vec2 uv = fragCoord.xy;
                    
                    // Copper bars as background
                    vec3 background = copperBars(uv);
                    
                    // Text with sine wave effect (no middle band constraint)
                    vec2 textUV = uv;
                    textUV.x = fract(textUV.x - iTime * 0.1);  // Scroll speed
                    textUV.y += 0.1 * sin(textUV.x * 6.0 + iTime * 3.0);  // Sine wave
                    
                    vec4 texColor = texture2D(iChannel0, textUV);
                    
                    // Mix text with copper bars
                    gl_FragColor = vec4(mix(background, vec3(1.0, 1.0, 0.0), texColor.a), 1.0);
                }
            "#,
        )?;

        let effect_program = link_program(&context, &effect_vertex_shader, &effect_fragment_shader)?;

        // Create a full-screen quad
        let vertices: [f32; 12] = [
            -1.0, -1.0,  // Bottom left
             1.0, -1.0,  // Bottom right
            -1.0,  1.0,  // Top left
            -1.0,  1.0,  // Top left
             1.0, -1.0,  // Bottom right
             1.0,  1.0,  // Top right
        ];

        let quad_buffer = context.create_buffer().unwrap();
        context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&quad_buffer));
        unsafe {
            let vert_array = js_sys::Float32Array::view(&vertices);
            context.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGlRenderingContext::STATIC_DRAW,
            );
        }

        // Create text texture
        let font = Font::from_bytes(FONT_DATA, fontdue::FontSettings::default())
            .map_err(|e| format!("Failed to load font: {:?}", e))?;
        let text = "PiRATE MiND STATiON   ";
        let scale = 128.0;
        let (text_texture, width, height) = create_text_texture(&context, text, &font, scale)?;
        web_sys::console::log_3(
            &"Text texture created:".into(),
            &width.into(),
            &height.into()
        );

        Ok(DemoEffect {
            context,
            copper_program,
            effect_program,
            text_texture,
            quad_buffer,
            time: 0.0,
            resolution: (canvas_width, canvas_height),
            text_scale: 1.0,
        })
    }

    pub fn render(&mut self) {
        // First render copper bars
        self.context.use_program(Some(&self.copper_program));
        let copper_time_loc = self.context.get_uniform_location(&self.copper_program, "time").unwrap();
        self.context.uniform1f(Some(&copper_time_loc), self.time);

        let position_loc = self.context.get_attrib_location(&self.copper_program, "position") as u32;
        self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&self.quad_buffer));
        self.context.vertex_attrib_pointer_with_i32(position_loc, 2, WebGlRenderingContext::FLOAT, false, 0, 0);
        self.context.enable_vertex_attrib_array(position_loc);
        
        self.context.draw_arrays(WebGlRenderingContext::TRIANGLES, 0, 6);

        // Then render the effect on top with blending
        self.context.enable(WebGlRenderingContext::BLEND);
        self.context.blend_func(
            WebGlRenderingContext::SRC_ALPHA,
            WebGlRenderingContext::ONE_MINUS_SRC_ALPHA,
        );

        self.context.use_program(Some(&self.effect_program));
        
        // Set time uniform
        if let Some(time_loc) = self.context.get_uniform_location(&self.effect_program, "iTime") {
            self.context.uniform1f(Some(&time_loc), self.time);
        }

        // Only set up texture
        self.context.active_texture(WebGlRenderingContext::TEXTURE0);
        self.context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&self.text_texture));
        
        if let Some(tex_loc) = self.context.get_uniform_location(&self.effect_program, "iChannel0") {
            self.context.uniform1i(Some(&tex_loc), 0);
        }

        // Draw fullscreen quad
        let position_loc = self.context.get_attrib_location(&self.effect_program, "position") as u32;
        self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&self.quad_buffer));
        self.context.vertex_attrib_pointer_with_i32(position_loc, 2, WebGlRenderingContext::FLOAT, false, 0, 0);
        self.context.enable_vertex_attrib_array(position_loc);
        
        self.context.draw_arrays(WebGlRenderingContext::TRIANGLES, 0, 6);

        // Cleanup
        self.context.disable(WebGlRenderingContext::BLEND);
        self.context.disable_vertex_attrib_array(position_loc);

        self.time += 0.016;
    }

    #[wasm_bindgen]
    pub fn resize(&mut self, width: i32, height: i32) {
        self.resolution = (width, height);
        self.context.viewport(0, 0, width, height);
    }

    #[wasm_bindgen]
    pub fn set_text_scale(&mut self, scale: f32) -> Result<(), JsValue> {
        self.text_scale = scale;
        // Recreate text texture with new scale
        let font = Font::from_bytes(FONT_DATA, fontdue::FontSettings::default())
            .map_err(|e| format!("Failed to load font: {:?}", e))?;
        let text = "PiRATE MiND STATiON   ";
        let base_scale = 48.0;
        let (new_texture, _, _) = create_text_texture(&self.context, text, &font, base_scale * scale)?;
        self.text_texture = new_texture;
        Ok(())
    }
}

fn compile_shader(
    context: &WebGlRenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

fn link_program(
    context: &WebGlRenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}

fn create_text_texture(context: &WebGlRenderingContext, text: &str, font: &Font, _scale: f32) -> Result<(WebGlTexture, i32, i32), JsValue> {
    let text_texture = context.create_texture().ok_or("Failed to create texture")?;
    context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&text_texture));
    
    // Use same dimensions that worked with checkerboard
    let width = 512;
    let height = 64;
    
    // Create bitmap
    let mut bitmap = vec![0u8; width * height];
    
    // Render text centered
    let scale = 48.0;
    let layouts: Vec<_> = text.chars().map(|c| font.metrics(c, scale)).collect();
    let total_width: f32 = layouts.iter().map(|l| l.advance_width).sum();
    
    // Center position - but flip vertically
    let start_x = ((width as f32 - total_width) / 2.0) as usize;
    let start_y = height - ((height as f32 / 2.0) as usize);
    
    // Render each character
    let mut x_pos = start_x;
    for (c, layout) in text.chars().zip(layouts.iter()) {
        let (_, char_bitmap) = font.rasterize(c, scale);
        
        // Copy character bitmap - flip vertically while copying
        for y in 0..layout.height {
            for x in 0..layout.width {
                let src_idx = y * layout.width + x;
                let dst_y = start_y - y;
                let dst_idx = dst_y * width + x_pos + x;
                if src_idx < char_bitmap.len() && dst_idx < bitmap.len() {
                    bitmap[dst_idx] = char_bitmap[src_idx];
                }
            }
        }
        x_pos += layout.advance_width as usize;
    }

    // Upload texture with same parameters that worked
    context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        WebGlRenderingContext::TEXTURE_2D,
        0,
        WebGlRenderingContext::ALPHA as i32,
        width as i32,
        height as i32,
        0,
        WebGlRenderingContext::ALPHA,
        WebGlRenderingContext::UNSIGNED_BYTE,
        Some(&bitmap),
    )?;

    // Keep same texture parameters
    context.tex_parameteri(
        WebGlRenderingContext::TEXTURE_2D,
        WebGlRenderingContext::TEXTURE_MIN_FILTER,
        WebGlRenderingContext::NEAREST as i32,
    );
    context.tex_parameteri(
        WebGlRenderingContext::TEXTURE_2D,
        WebGlRenderingContext::TEXTURE_MAG_FILTER,
        WebGlRenderingContext::NEAREST as i32,
    );
    context.tex_parameteri(
        WebGlRenderingContext::TEXTURE_2D,
        WebGlRenderingContext::TEXTURE_WRAP_S,
        WebGlRenderingContext::REPEAT as i32,
    );
    context.tex_parameteri(
        WebGlRenderingContext::TEXTURE_2D,
        WebGlRenderingContext::TEXTURE_WRAP_T,
        WebGlRenderingContext::CLAMP_TO_EDGE as i32,
    );

    Ok((text_texture, width as i32, height as i32))
} 