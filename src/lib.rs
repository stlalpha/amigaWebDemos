use wasm_bindgen::prelude::*;
use web_sys::{WebGlRenderingContext, WebGlProgram, WebGlShader, HtmlCanvasElement};
use wasm_bindgen::JsCast;

#[wasm_bindgen]
pub struct DemoEffect {
    context: WebGlRenderingContext,
    copper_program: WebGlProgram,
    text_program: WebGlProgram,
    time: f32,
    scroll_offset: f32,
    canvas_width: i32,
    canvas_height: i32,
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
                uniform float time;
                uniform float scroll_offset;
                
                void main() {
                    vec4 pos = position;
                    // More pronounced sine wave effect
                    pos.y += sin(time * 3.0 + position.x * 4.0) * 0.15;
                    // Apply scrolling
                    pos.x += scroll_offset;
                    gl_Position = pos;
                }
            "#,
        )?;

        let text_fragment_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
                precision mediump float;
                void main() {
                    gl_FragColor = vec4(1.0, 1.0, 0.0, 1.0); // Yellow text
                }
            "#,
        )?;

        let copper_program = link_program(&context, &copper_vertex_shader, &copper_fragment_shader)?;
        let text_program = link_program(&context, &text_vertex_shader, &text_fragment_shader)?;

        Ok(DemoEffect {
            context,
            copper_program,
            text_program,
            time: 0.0,
            scroll_offset: 1.0,
            canvas_width,
            canvas_height,
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
        
        let text_time_location = self.context.get_uniform_location(&self.text_program, "time");
        self.context.uniform1f(text_time_location.as_ref(), self.time);
        
        let scroll_location = self.context.get_uniform_location(&self.text_program, "scroll_offset");
        self.context.uniform1f(scroll_location.as_ref(), self.scroll_offset);

        // Render each letter
        let text = "PiRATE MiND STATiON";
        for (i, _) in text.chars().enumerate() {
            let letter_width = 0.15; // Width of each letter
            let spacing = 0.02; // Space between letters
            let start_x = (i as f32) * (letter_width + spacing);
            
            let text_vertices: [f32; 12] = [
                start_x, -0.1, 
                start_x + letter_width, -0.1, 
                start_x, 0.1,
                start_x, 0.1, 
                start_x + letter_width, -0.1, 
                start_x + letter_width, 0.1
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
            self.context.vertex_attrib_pointer_with_i32(text_position, 2, WebGlRenderingContext::FLOAT, false, 0, 0);
            self.context.enable_vertex_attrib_array(text_position);
            
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