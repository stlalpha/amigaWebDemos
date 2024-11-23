use wasm_bindgen::prelude::*;
use web_sys::{WebGlRenderingContext, WebGlProgram, WebGlShader, HtmlCanvasElement};
use web_sys::WebGlBuffer;
use js_sys::Float32Array;

#[wasm_bindgen]
pub struct DemoEffect {
    gl: WebGlRenderingContext,
    program: WebGlProgram,
    position_buffer: WebGlBuffer,
    start_time: f64,
    last_time: f64,
    frame_count: i32,
}

#[wasm_bindgen]
impl DemoEffect {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<DemoEffect, JsValue> {
        let document = web_sys::window()
            .unwrap()
            .document()
            .unwrap();
        
        let canvas = document
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()?;

        let gl = canvas
            .get_context("webgl")?
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()?;

        // Compile shaders and create program
        let vert_shader = compile_shader(
            &gl,
            WebGlRenderingContext::VERTEX_SHADER,
            include_str!("shaders/starfield.vert"),
        )?;

        let frag_shader = compile_shader(
            &gl,
            WebGlRenderingContext::FRAGMENT_SHADER,
            include_str!("shaders/starfield.frag"),
        )?;

        let program = link_program(&gl, &vert_shader, &frag_shader)?;
        gl.use_program(Some(&program));

        // Create position buffer
        let position_buffer = gl.create_buffer().ok_or("Failed to create buffer")?;
        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&position_buffer));

        let vertices: [f32; 8] = [
            -1.0, -1.0,
             1.0, -1.0,
            -1.0,  1.0,
             1.0,  1.0,
        ];

        let vert_array = Float32Array::from(&vertices[..]);

        gl.buffer_data_with_array_buffer_view(
            WebGlRenderingContext::ARRAY_BUFFER,
            &vert_array,
            WebGlRenderingContext::STATIC_DRAW,
        );

        let position_attribute_location = gl.get_attrib_location(&program, "a_position") as u32;
        gl.enable_vertex_attrib_array(position_attribute_location);
        gl.vertex_attrib_pointer_with_i32(position_attribute_location, 2, WebGlRenderingContext::FLOAT, false, 0, 0);

        let now = web_sys::window()
            .unwrap()
            .performance()
            .unwrap()
            .now();

        Ok(DemoEffect {
            gl,
            program,
            position_buffer,
            start_time: now,
            last_time: now,
            frame_count: 0,
        })
    }

    pub fn render(&mut self) -> Result<(), JsValue> {
        let now = web_sys::window()
            .unwrap()
            .performance()
            .unwrap()
            .now();

        let time = (now - self.start_time) * 0.001; // Convert to seconds
        let delta_time = (now - self.last_time) * 0.001;
        self.last_time = now;
        self.frame_count += 1;

        let canvas = self.gl.canvas().unwrap().dyn_into::<HtmlCanvasElement>()?;
        let width = canvas.client_width() as f32;
        let height = canvas.client_height() as f32;

        // Update uniforms
        let resolution_location = self.gl.get_uniform_location(&self.program, "iResolution");
        self.gl.uniform3f(resolution_location.as_ref(), width, height, 1.0);

        let time_location = self.gl.get_uniform_location(&self.program, "iTime");
        self.gl.uniform1f(time_location.as_ref(), time as f32);

        let time_delta_location = self.gl.get_uniform_location(&self.program, "iTimeDelta");
        self.gl.uniform1f(time_delta_location.as_ref(), delta_time as f32);

        let frame_rate_location = self.gl.get_uniform_location(&self.program, "iFrameRate");
        self.gl.uniform1f(frame_rate_location.as_ref(), 1.0 / delta_time as f32);

        let frame_location = self.gl.get_uniform_location(&self.program, "iFrame");
        self.gl.uniform1i(frame_location.as_ref(), self.frame_count);

        // Draw
        self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
        self.gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);
        self.gl.draw_arrays(WebGlRenderingContext::TRIANGLE_STRIP, 0, 4);

        Ok(())
    }
}

fn compile_shader(
    gl: &WebGlRenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if gl.get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

fn link_program(
    gl: &WebGlRenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = gl
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    gl.attach_shader(&program, vert_shader);
    gl.attach_shader(&program, frag_shader);
    gl.link_program(&program);

    if gl.get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
} 