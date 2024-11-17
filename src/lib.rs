use wasm_bindgen::prelude::*;
use web_sys::{WebGlRenderingContext, WebGlProgram, WebGlShader, WebGlTexture};

#[wasm_bindgen]
pub struct DemoEffect {
    context: WebGlRenderingContext,
    program: WebGlProgram,
    text_program: WebGlProgram,
    time: f32,
    font_texture: WebGlTexture,
    scroll_text: String,
    scroll_offset: f32,
}

#[wasm_bindgen]
impl DemoEffect {
    pub fn new(canvas_id: &str) -> Result<DemoEffect, JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id(canvas_id).unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;
        
        let context = canvas
            .get_context("webgl")?
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()?;
            
        // Vertex shader for copper bars
        let vertex_shader = compile_shader(
            &context,
            WebGlRenderingContext::VERTEX_SHADER,
            r#"
                attribute vec4 position;
                void main() {
                    gl_Position = position;
                }
            "#,
        )?;

        // Fragment shader for copper bars
        let fragment_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
                precision mediump float;
                uniform float time;
                
                void main() {
                    float y = gl_FragCoord.y;
                    float wave = sin(y * 0.1 + time) * 0.5 + 0.5;
                    vec3 color = vec3(wave, wave * 0.5, 0.0);
                    gl_FragColor = vec4(color, 1.0);
                }
            "#,
        )?;

        let program = link_program(&context, &vertex_shader, &fragment_shader)?;
        context.use_program(Some(&program));

        // Add new vertex shader for text
        let text_vertex_shader = compile_shader(
            &context,
            WebGlRenderingContext::VERTEX_SHADER,
            r#"
                attribute vec4 position;
                attribute vec2 tex_coords;
                varying vec2 v_tex_coords;
                uniform float time;
                uniform float scroll_offset;
                
                void main() {
                    vec4 pos = position;
                    // Apply sine wave effect
                    pos.y += sin(time * 2.0 + pos.x * 3.0) * 0.1;
                    // Apply scrolling
                    pos.x += scroll_offset;
                    gl_Position = pos;
                    v_tex_coords = tex_coords;
                }
            "#,
        )?;

        // Add fragment shader for text
        let text_fragment_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
                precision mediump float;
                varying vec2 v_tex_coords;
                uniform sampler2D font_texture;
                
                void main() {
                    vec4 color = texture2D(font_texture, v_tex_coords);
                    // Make text yellow
                    gl_FragColor = vec4(1.0, 1.0, 0.0, color.a);
                }
            "#,
        )?;

        let text_program = link_program(&context, &text_vertex_shader, &text_fragment_shader)?;
        
        // Create font texture
        let font_texture = create_font_texture(&context)?;

        Ok(DemoEffect {
            context,
            program,
            text_program,
            time: 0.0,
            font_texture,
            scroll_text: "HELLO WORLD * AMIGA FOREVER * GREETINGS TO ALL DEMO MAKERS! * ".to_string(),
            scroll_offset: 1.0,
        })
    }

    pub fn render(&mut self) {
        self.time += 0.016; // Assume 60fps
        
        // Clear screen
        self.context.clear_color(0.0, 0.0, 0.0, 1.0);
        self.context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);
        
        // Update time uniform
        let time_location = self.context.get_uniform_location(&self.program, "time");
        self.context.uniform1f(time_location.as_ref(), self.time);
        
        // Draw fullscreen quad
        let vertices: [f32; 12] = [
            -1.0, -1.0, 1.0, -1.0, -1.0, 1.0,
            -1.0, 1.0, 1.0, -1.0, 1.0, 1.0
        ];
        
        let buffer = self.context.create_buffer().unwrap();
        self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&buffer));
        
        unsafe {
            let vert_array = js_sys::Float32Array::view(&vertices);
            self.context.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGlRenderingContext::STATIC_DRAW,
            );
        }
        
        let position = self.context.get_attrib_location(&self.program, "position") as u32;
        self.context.vertex_attrib_pointer_with_i32(position, 2, WebGlRenderingContext::FLOAT, false, 0, 0);
        self.context.enable_vertex_attrib_array(position);
        
        self.context.draw_arrays(WebGlRenderingContext::TRIANGLES, 0, 6);

        // Render scrolling text
        self.context.use_program(Some(&self.text_program));
        
        // Update uniforms
        let time_location = self.context.get_uniform_location(&self.text_program, "time");
        self.context.uniform1f(time_location.as_ref(), self.time);
        
        self.scroll_offset -= 0.01;
        let scroll_location = self.context.get_uniform_location(&self.text_program, "scroll_offset");
        self.context.uniform1f(scroll_location.as_ref(), self.scroll_offset);

        // Reset scroll when text is off screen
        if self.scroll_offset < -2.0 {
            self.scroll_offset = 1.0;
        }

        // Render text characters
        self.render_text();
    }

    fn render_text(&self) {
        // Create vertices for each character
        for (i, c) in self.scroll_text.chars().enumerate() {
            let x = i as f32 * 0.1;  // character spacing
            let vertices = create_character_quad(x, 0.0, c);
            // ... render character using vertices
        }
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

fn create_character_quad(x: f32, y: f32, c: char) -> Vec<f32> {
    let char_width = 0.1;
    let char_height = 0.2;
    
    // Calculate texture coordinates based on character
    let tx = ((c as u32 - 32) % 16) as f32 / 16.0;
    let ty = ((c as u32 - 32) / 16) as f32 / 16.0;
    
    vec![
        x, y, tx, ty,
        x + char_width, y, tx + 1.0/16.0, ty,
        x, y + char_height, tx, ty + 1.0/16.0,
        x + char_width, y + char_height, tx + 1.0/16.0, ty + 1.0/16.0,
    ]
}

fn create_font_texture(context: &WebGlRenderingContext) -> Result<WebGlTexture, JsValue> {
    let texture = context.create_texture().unwrap();
    context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&texture));
    
    // Here you would normally load a font texture
    // For this example, we'll create a simple bitmap font
    let font_data = create_bitmap_font();
    
    // Upload texture data
    unsafe {
        let tex_data = js_sys::Uint8Array::view(&font_data);
        context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
            WebGlRenderingContext::TEXTURE_2D,
            0,
            WebGlRenderingContext::RGBA as i32,
            16,  // width
            16,  // height
            0,
            WebGlRenderingContext::RGBA,
            WebGlRenderingContext::UNSIGNED_BYTE,
            Some(&tex_data),
        )?;
    }
    
    Ok(texture)
}

fn create_bitmap_font() -> Vec<u8> {
    // Create a simple 16x16 bitmap font texture
    // This is a placeholder - you'd want to create proper font data
    vec![255; 16 * 16 * 4]
} 