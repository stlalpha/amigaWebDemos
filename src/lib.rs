use wasm_bindgen::prelude::*;
use web_sys::{WebGlRenderingContext, WebGlProgram, WebGlShader, WebGlTexture};
use fontdue::Font;

const FONT_DATA: &[u8] = include_bytes!("topaza1200.ttf");

#[wasm_bindgen]
pub struct DemoEffect {
    context: WebGlRenderingContext,
    copper_program: WebGlProgram,
    text_program: WebGlProgram,
    time: f32,
    scroll_offset: f32,
    canvas_width: i32,
    canvas_height: i32,
    font_texture: WebGlTexture,
    char_positions: Vec<(f32, f32, f32, f32)>,
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
                    vec3 color = vec3(wave, wave * 0.5, 0.0);
                    gl_FragColor = vec4(color, 1.0);
                }
            "#,
        )?;

        // Text shader program
        let text_vertex_shader = compile_shader(
            &context,
            WebGlRenderingContext::VERTEX_SHADER,
            r#"
                attribute vec4 position;
                attribute vec2 texcoord;
                uniform float time;
                uniform float scroll_offset;
                varying vec2 v_texcoord;
                
                void main() {
                    vec4 pos = position;
                    pos.y += sin(time * 3.0 + position.x * 4.0) * 0.15;
                    pos.x += scroll_offset;
                    v_texcoord = texcoord;
                    gl_Position = pos;
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
                    gl_FragColor = vec4(1.0, 1.0, 0.0, texColor.a);
                }
            "#,
        )?;

        let copper_program = link_program(&context, &copper_vertex_shader, &copper_fragment_shader)?;
        let text_program = link_program(&context, &text_vertex_shader, &text_fragment_shader)?;

        let (font_texture, char_positions) = init_font_texture(&context)?;

        Ok(DemoEffect {
            context,
            copper_program,
            text_program,
            time: 0.0,
            scroll_offset: 1.0,
            canvas_width,
            canvas_height,
            font_texture,
            char_positions,
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
        self.scroll_offset -= 0.005;
        if self.scroll_offset < -3.0 {
            self.scroll_offset = 1.0;
        }

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

        // Render scrolling text
        self.context.use_program(Some(&self.text_program));
        self.context.active_texture(WebGlRenderingContext::TEXTURE0);
        self.context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&self.font_texture));
        self.context.uniform1i(
            self.context.get_uniform_location(&self.text_program, "u_texture").as_ref(),
            0,
        );

        let text_time_location = self.context.get_uniform_location(&self.text_program, "time");
        self.context.uniform1f(text_time_location.as_ref(), self.time);

        let scroll_location = self.context.get_uniform_location(&self.text_program, "scroll_offset");
        self.context.uniform1f(scroll_location.as_ref(), self.scroll_offset);

        // Render each letter
        let text = "PiRATE MiND STATiON";
        for (i, c) in text.chars().enumerate() {
            let letter_width = 0.15;
            let spacing = 0.02;
            let start_x = (i as f32) * (letter_width + spacing);
            
            let (u1, v1, u2, v2) = get_char_uvs(c, &self.char_positions);
            
            let text_vertices: [f32; 24] = [
                // Position (x,y)    // Texcoords (u,v)
                start_x, -0.1,       u1, v1,
                start_x + letter_width, -0.1,  u2, v1,
                start_x, 0.1,        u1, v2,
                start_x, 0.1,        u1, v2,
                start_x + letter_width, -0.1,  u2, v1,
                start_x + letter_width, 0.1,   u2, v2
            ];
            
            let text_buffer = self.context.create_buffer().unwrap();
            self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&text_buffer));
            
            unsafe {
                let vert_array = js_sys::Float32Array::view(&text_vertices);
                self.context.buffer_data_with_array_buffer_view(
                    WebGlRenderingContext::ARRAY_BUFFER,
                    &vert_array,
                    WebGlRenderingContext::STATIC_DRAW,
                );
            }
            
            let text_position = self.context.get_attrib_location(&self.text_program, "position") as u32;
            self.context.vertex_attrib_pointer_with_i32(
                text_position,
                2,
                WebGlRenderingContext::FLOAT,
                false,
                4 * 4,  // stride: 4 values per vertex
                0,      // offset for position
            );
            self.context.enable_vertex_attrib_array(text_position);
            
            let texcoord_loc = self.context.get_attrib_location(&self.text_program, "texcoord") as u32;
            self.context.vertex_attrib_pointer_with_i32(
                texcoord_loc,
                2,
                WebGlRenderingContext::FLOAT,
                false,
                4 * 4,  // stride: 4 values per vertex
                2 * 4,  // offset for texcoords
            );
            self.context.enable_vertex_attrib_array(texcoord_loc);
            
            self.context.draw_arrays(WebGlRenderingContext::TRIANGLES, 0, 6);
        }
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

fn init_font_texture(context: &WebGlRenderingContext) -> Result<(WebGlTexture, Vec<(f32, f32, f32, f32)>), JsValue> {
    web_sys::console::log_1(&"Starting font initialization".into());
    
    let font = Font::from_bytes(FONT_DATA, fontdue::FontSettings::default())
        .map_err(|e| JsValue::from_str(&format!("Failed to load font: {}", e)))?;
    
    // Increased atlas size further
    let atlas_size = 512;  // Much larger atlas
    let mut atlas = vec![0u8; atlas_size * atlas_size * 4];
    let mut char_positions = vec![(0.0, 0.0, 0.0, 0.0); 128];
    
    let mut x = 0;
    let mut y = 0;
    let char_size = 32;

    // Log initial setup
    web_sys::console::log_1(&format!("Atlas size: {}x{}, char size: {}", 
        atlas_size, atlas_size, char_size).into());

    // Calculate how many characters we can fit per row
    let chars_per_row = atlas_size / char_size;
    web_sys::console::log_1(&format!("Can fit {} chars per row", chars_per_row).into());

    for c in 32..128u8 {
        // Log current position
        web_sys::console::log_1(&format!("Processing char '{}' at position ({}, {})", 
            c as char, x, y).into());

        let (metrics, bitmap) = font.rasterize(c as char, 24.0);
        
        // Log metrics for each character
        web_sys::console::log_1(&format!("Char '{}' metrics: {}x{}", 
            c as char, metrics.width, metrics.height).into());

        if x + char_size > atlas_size {
            web_sys::console::log_1(&format!("Row full at char '{}', moving to next row", 
                c as char).into());
            x = 0;
            y += char_size;
        }
        
        if y + char_size > atlas_size {
            web_sys::console::log_1(&format!("Atlas full at char '{}', y={}, atlas_size={}", 
                c as char, y, atlas_size).into());
            return Err(JsValue::from_str(&format!(
                "Atlas size exceeded at char '{}'. Position: ({}, {}), Atlas size: {}", 
                c as char, x, y, atlas_size)));
        }

        let u1 = x as f32 / atlas_size as f32;
        let v1 = y as f32 / atlas_size as f32;
        let u2 = (x + char_size) as f32 / atlas_size as f32;
        let v2 = (y + char_size) as f32 / atlas_size as f32;
        char_positions[c as usize] = (u1, v1, u2, v2);

        // Copy bitmap with bounds checking and logging
        for (i, &pixel) in bitmap.iter().enumerate() {
            let bx = i % metrics.width;
            let by = i / metrics.width;
            let ax = x + bx + (char_size - metrics.width) / 2;
            let ay = y + by + (char_size - metrics.height) / 2;
            
            if ax >= atlas_size || ay >= atlas_size {
                web_sys::console::log_1(&format!("Out of bounds at ({}, {})", ax, ay).into());
                continue;
            }

            let atlas_idx = (ay * atlas_size + ax) * 4;
            if atlas_idx + 3 >= atlas.len() {
                web_sys::console::log_1(&"Atlas index out of bounds".into());
                continue;
            }

            atlas[atlas_idx] = pixel;
            atlas[atlas_idx + 1] = pixel;
            atlas[atlas_idx + 2] = pixel;
            atlas[atlas_idx + 3] = pixel;
        }
        
        x += char_size;
    }

    web_sys::console::log_1(&"Atlas generation complete".into());

    let texture = context.create_texture().unwrap();
    context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&texture));
    
    context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        WebGlRenderingContext::TEXTURE_2D,
        0,
        WebGlRenderingContext::RGBA as i32,
        atlas_size as i32,
        atlas_size as i32,
        0,
        WebGlRenderingContext::RGBA,
        WebGlRenderingContext::UNSIGNED_BYTE,
        Some(&atlas),
    )?;

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

    web_sys::console::log_1(&"Font initialization complete".into());
    Ok((texture, char_positions))
}

fn get_char_uvs(c: char, char_positions: &[(f32, f32, f32, f32)]) -> (f32, f32, f32, f32) {
    let (u1, v1, u2, v2) = char_positions[c as usize];
    // Flip v coordinates by returning them in reverse order
    (u1, v2, u2, v1)  // Changed from (u1, v1, u2, v2)
} 