use wasm_bindgen::prelude::*;
use web_sys::{WebGlRenderingContext, WebGlProgram, WebGlShader, WebGlTexture, WebGlBuffer};
use fontdue::Font;

const FONT_DATA: &[u8] = include_bytes!("topaza1200.ttf");

#[wasm_bindgen]
pub struct DemoEffect {
    context: WebGlRenderingContext,
    copper_program: WebGlProgram,
    text_program: WebGlProgram,
    text_texture: WebGlTexture,
    text_vbo: WebGlBuffer,
    text_tbo: WebGlBuffer,
    time: f32,
    scroll_offset: f32,
    canvas_width: i32,
    canvas_height: i32,
    start_delay: f32,
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

        // Get initial canvas size
        let canvas_width = canvas.width() as i32;
        let canvas_height = canvas.height() as i32;
        
        // Copper bars shader program
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

        // Add text shader program setup
        let text_vertex_shader = compile_shader(
            &context,
            WebGlRenderingContext::VERTEX_SHADER,
            r#"
                attribute vec4 position;
                attribute vec2 texcoord;
                varying vec2 v_texcoord;
                uniform float scroll_offset;
                uniform float time;
                
                void main() {
                    vec4 pos = position;
                    pos.y += sin(time * 1.5 + pos.x * 0.5) * 0.08;
                    pos.x += scroll_offset;
                    gl_Position = pos;
                    v_texcoord = texcoord;
                }
            "#,
        )?;

        let text_fragment_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
                precision mediump float;
                varying vec2 v_texcoord;
                uniform sampler2D u_texture;
                
                void main() {
                    vec4 texColor = texture2D(u_texture, v_texcoord);
                    gl_FragColor = vec4(1.0, 1.0, 0.0, texColor.r);
                }
            "#,
        )?;

        let text_program = link_program(&context, &text_vertex_shader, &text_fragment_shader)?;

        // Load font
        let font = Font::from_bytes(FONT_DATA, fontdue::FontSettings::default())
            .map_err(|e| format!("Failed to load font: {:?}", e))?;
        
        // Create text texture
        let text = "PiRATE MiND STATiON    ";
        let scale = 64.0;
        let (text_texture, _width, _height) = create_text_texture(&context, text, &font, scale)?;

        // Adjust text vertices based on aspect ratio
        let text_vertices: [f32; 12] = [
            0.0, -0.15,   // Bottom left
            2.0, -0.15,   // Bottom right
            0.0,  0.15,   // Top left
            0.0,  0.15,   // Top left
            2.0, -0.15,   // Bottom right
            2.0,  0.15,   // Top right
        ];

        let text_texcoords: [f32; 12] = [
            0.0, 1.0,
            1.0, 1.0,
            0.0, 0.0,
            0.0, 0.0,
            1.0, 1.0,
            1.0, 0.0,
        ];

        let text_vbo = context.create_buffer().unwrap();
        let text_tbo = context.create_buffer().unwrap();

        // Initialize vertex buffer
        context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&text_vbo));
        unsafe {
            let vert_array = js_sys::Float32Array::view(&text_vertices);
            context.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGlRenderingContext::STATIC_DRAW,
            );
        }

        // Initialize texture coordinate buffer
        context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&text_tbo));
        unsafe {
            let tex_array = js_sys::Float32Array::view(&text_texcoords);
            context.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &tex_array,
                WebGlRenderingContext::STATIC_DRAW,
            );
        }

        Ok(DemoEffect {
            context,
            copper_program,
            text_program,
            text_texture,
            text_vbo,
            text_tbo,
            time: 0.0,
            scroll_offset: 1.0,
            canvas_width,
            canvas_height,
            start_delay: 1.0,
        })
    }

    pub fn render(&mut self) {
        // Update viewport if canvas size changed
        let canvas = self.context
            .canvas()
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
            
        let client_width = canvas.client_width();
        let client_height = canvas.client_height();
        
        if client_width != self.canvas_width || client_height != self.canvas_height {
            canvas.set_width(client_width as u32);
            canvas.set_height(client_height as u32);
            self.resize(client_width, client_height);
        }

        self.time += 0.016;

        self.context.clear_color(0.0, 0.0, 0.0, 1.0);
        self.context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        // Render copper bars
        self.context.use_program(Some(&self.copper_program));
        let time_location = self.context.get_uniform_location(&self.copper_program, "time");
        self.context.uniform1f(time_location.as_ref(), self.time);
        
        // Draw copper bars
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
        
        let position = self.context.get_attrib_location(&self.copper_program, "position") as u32;
        self.context.vertex_attrib_pointer_with_i32(position, 2, WebGlRenderingContext::FLOAT, false, 0, 0);
        self.context.enable_vertex_attrib_array(position);
        
        self.context.draw_arrays(WebGlRenderingContext::TRIANGLES, 0, 6);

        // Only start scrolling after delay
        if self.start_delay > 0.0 {
            self.start_delay -= 0.016;  // Assuming 60fps
            return;
        }

        // Update scroll position
        self.scroll_offset -= 0.005;
        if self.scroll_offset < -3.0 {
            self.scroll_offset = 1.0;
        }

        // Render scrolling text
        self.context.use_program(Some(&self.text_program));
        
        // Enable blending for text transparency
        self.context.enable(WebGlRenderingContext::BLEND);
        self.context.blend_func(
            WebGlRenderingContext::SRC_ALPHA,
            WebGlRenderingContext::ONE_MINUS_SRC_ALPHA,
        );

        // Set uniforms
        let time_loc = self.context.get_uniform_location(&self.text_program, "time");
        let scroll_loc = self.context.get_uniform_location(&self.text_program, "scroll_offset");
        self.context.uniform1f(time_loc.as_ref(), self.time);
        self.context.uniform1f(scroll_loc.as_ref(), self.scroll_offset);

        // Bind texture
        self.context.active_texture(WebGlRenderingContext::TEXTURE0);
        self.context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&self.text_texture));
        let sampler_loc = self.context.get_uniform_location(&self.text_program, "u_texture");
        self.context.uniform1i(sampler_loc.as_ref(), 0);

        // Set up vertex attributes
        let position_loc = self.context.get_attrib_location(&self.text_program, "position") as u32;
        let texcoord_loc = self.context.get_attrib_location(&self.text_program, "texcoord") as u32;

        // Bind and enable vertex position
        self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&self.text_vbo));
        self.context.vertex_attrib_pointer_with_i32(
            position_loc,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            0,
            0,
        );
        self.context.enable_vertex_attrib_array(position_loc);

        // Bind and enable texture coordinates
        self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&self.text_tbo));
        self.context.vertex_attrib_pointer_with_i32(
            texcoord_loc,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            0,
            0,
        );
        self.context.enable_vertex_attrib_array(texcoord_loc);

        // Draw the text
        self.context.draw_arrays(WebGlRenderingContext::TRIANGLES, 0, 6);

        // Clean up
        self.context.disable(WebGlRenderingContext::BLEND);
        self.context.disable_vertex_attrib_array(position_loc);
        self.context.disable_vertex_attrib_array(texcoord_loc);
    }

    #[wasm_bindgen]
    pub fn resize(&mut self, width: i32, height: i32) {
        // Get the actual canvas element
        let canvas = self.context
            .canvas()
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        
        // Set both the canvas size and viewport
        canvas.set_width(width as u32);
        canvas.set_height(height as u32);
        self.canvas_width = width;
        self.canvas_height = height;
        self.context.viewport(0, 0, width, height);
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

fn create_text_texture(context: &WebGlRenderingContext, text: &str, font: &Font, scale: f32) -> Result<(WebGlTexture, i32, i32), JsValue> {
    let text_texture = context.create_texture().ok_or("Failed to create texture")?;
    context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&text_texture));
    
    // Calculate total width and get max height
    let mut total_width = 0;
    let mut max_height = 0;
    let layouts: Vec<_> = text.chars().map(|c| font.metrics(c, scale)).collect();
    for layout in &layouts {
        total_width += layout.advance_width as i32;
        max_height = max_height.max(layout.height);
    }
    
    // Create bitmap
    let mut bitmap = vec![0u8; (total_width * max_height as i32) as usize];
    
    // Render each character
    let mut x_offset = 0;
    for (c, layout) in text.chars().zip(layouts.iter()) {
        let (_, char_bitmap) = font.rasterize(c, scale);
        
        // Copy character bitmap to the correct position
        for y in 0..layout.height {
            for x in 0..layout.width {
                let src_idx = y * layout.width + x;
                let dst_idx = (y * total_width as usize + x + x_offset) as usize;
                if src_idx < char_bitmap.len() && dst_idx < bitmap.len() {
                    bitmap[dst_idx] = char_bitmap[src_idx];
                }
            }
        }
        x_offset += layout.advance_width as usize;
    }
    
    // Upload texture data
    unsafe {
        let tex_data = js_sys::Uint8Array::view(&bitmap);
        context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
            WebGlRenderingContext::TEXTURE_2D,
            0,
            WebGlRenderingContext::LUMINANCE as i32,
            total_width,
            max_height as i32,
            0,
            WebGlRenderingContext::LUMINANCE,
            WebGlRenderingContext::UNSIGNED_BYTE,
            Some(&tex_data),
        )?;
    }
    
    // Set texture parameters
    context.tex_parameteri(
        WebGlRenderingContext::TEXTURE_2D,
        WebGlRenderingContext::TEXTURE_MIN_FILTER,
        WebGlRenderingContext::LINEAR as i32,
    );
    context.tex_parameteri(
        WebGlRenderingContext::TEXTURE_2D,
        WebGlRenderingContext::TEXTURE_WRAP_S,
        WebGlRenderingContext::CLAMP_TO_EDGE as i32,
    );
    context.tex_parameteri(
        WebGlRenderingContext::TEXTURE_2D,
        WebGlRenderingContext::TEXTURE_WRAP_T,
        WebGlRenderingContext::CLAMP_TO_EDGE as i32,
    );
    
    Ok((text_texture, total_width, max_height as i32))
} 