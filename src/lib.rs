use wasm_bindgen::prelude::*;
use web_sys::{WebGlRenderingContext, WebGlProgram, WebGlShader, WebGlTexture};

#[wasm_bindgen]
pub struct DemoEffect {
    context: WebGlRenderingContext,
    copper_program: WebGlProgram,
    text_program: WebGlProgram,
    time: f32,
    scroll_offset: f32,
    font_texture: WebGlTexture,
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
                attribute vec2 tex_coord;
                uniform float time;
                uniform float scroll_offset;
                varying vec2 v_tex_coord;
                
                void main() {
                    vec4 pos = position;
                    pos.y += sin(time * 3.0 + position.x * 4.0) * 0.15;
                    pos.x += scroll_offset;
                    gl_Position = pos;
                    v_tex_coord = tex_coord;
                }
            "#,
        )?;

        let text_fragment_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
                precision mediump float;
                varying vec2 v_tex_coord;
                uniform sampler2D u_texture;
                
                void main() {
                    vec4 texel = texture2D(u_texture, v_tex_coord);
                    gl_FragColor = vec4(1.0, 1.0, 0.0, texel.a); // Yellow text
                }
            "#,
        )?;

        let copper_program = link_program(&context, &copper_vertex_shader, &copper_fragment_shader)?;
        let text_program = link_program(&context, &text_vertex_shader, &text_fragment_shader)?;

        // Create and initialize font texture
        let font_texture = create_font_texture(&context)?;

        Ok(DemoEffect {
            context,
            copper_program,
            text_program,
            time: 0.0,
            scroll_offset: 1.0,
            font_texture,
        })
    }

    pub fn render(&mut self) {
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

        // Text rendering
        self.context.use_program(Some(&self.text_program));
        
        // Bind texture
        self.context.active_texture(WebGlRenderingContext::TEXTURE0);
        self.context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&self.font_texture));
        let sampler_location = self.context.get_uniform_location(&self.text_program, "u_texture");
        self.context.uniform1i(sampler_location.as_ref(), 0);

        let text = "PiRATE MiND STATiON";
        for (i, c) in text.chars().enumerate() {
            let (vertices, tex_coords) = create_character_geometry(i as f32, c);
            
            // Position buffer
            let position_buffer = self.context.create_buffer().unwrap();
            self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&position_buffer));
            unsafe {
                let vert_array = js_sys::Float32Array::view(&vertices);
                self.context.buffer_data_with_array_buffer_view(
                    WebGlRenderingContext::ARRAY_BUFFER,
                    &vert_array,
                    WebGlRenderingContext::STATIC_DRAW,
                );
            }
            
            let position_loc = self.context.get_attrib_location(&self.text_program, "position") as u32;
            self.context.vertex_attrib_pointer_with_i32(position_loc, 2, WebGlRenderingContext::FLOAT, false, 0, 0);
            self.context.enable_vertex_attrib_array(position_loc);

            // Texture coordinate buffer
            let tex_coord_buffer = self.context.create_buffer().unwrap();
            self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&tex_coord_buffer));
            unsafe {
                let tex_array = js_sys::Float32Array::view(&tex_coords);
                self.context.buffer_data_with_array_buffer_view(
                    WebGlRenderingContext::ARRAY_BUFFER,
                    &tex_array,
                    WebGlRenderingContext::STATIC_DRAW,
                );
            }
            
            let tex_loc = self.context.get_attrib_location(&self.text_program, "tex_coord") as u32;
            self.context.vertex_attrib_pointer_with_i32(tex_loc, 2, WebGlRenderingContext::FLOAT, false, 0, 0);
            self.context.enable_vertex_attrib_array(tex_loc);

            self.context.draw_arrays(WebGlRenderingContext::TRIANGLES, 0, 6);
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

fn create_character_geometry(index: f32, c: char) -> (Vec<f32>, Vec<f32>) {
    let char_width = 0.15;
    let char_height = 0.2;
    let x = index * (char_width * 1.2); // Add 20% spacing between characters
    
    // Character quad vertices
    let vertices = vec![
        x, -0.1,
        x + char_width, -0.1,
        x, char_height - 0.1,
        x, char_height - 0.1,
        x + char_width, -0.1,
        x + char_width, char_height - 0.1,
    ];

    // Calculate texture coordinates based on character
    let char_index = (c as u32).saturating_sub(32) as f32; // ASCII offset
    let tex_x = (char_index % 16.0) / 16.0;
    let tex_y = (char_index / 16.0).floor() / 16.0;
    let tex_width = 1.0 / 16.0;
    let tex_height = 1.0 / 16.0;

    let tex_coords = vec![
        tex_x, tex_y + tex_height,
        tex_x + tex_width, tex_y + tex_height,
        tex_x, tex_y,
        tex_x, tex_y,
        tex_x + tex_width, tex_y + tex_height,
        tex_x + tex_width, tex_y,
    ];

    (vertices, tex_coords)
}

const TOPAZ_FONT_BITMAP: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAIAAAACACAYAAADDPmHLAAAABHNCSVQICAgIfAhkiAAAAAlwSFlzAAALEwAACxMBAJqcGAAAAu5JREFUeJzt3UFuE0EUhOF/bIlFuEZOw4Y9C/ZcB27DDdhwDrYcICshJZGHhQUUwii2xx6/mq6vktWolV7p1XTN9IxnZgYAAAAAAAAAAAAAAAAAAAAAAAAAgP/KrPsCUP7g9/f3ZmZWVVXVw8PDdK7/rqqenp6qqurhdrv9M7O367ruC/jlzOyjeH+/5UwJYGbW7vf7b2b2xd3Pq6pqmqaPVVWdTqfNuq77+Xx+c3f/WFV1Pp+rqmq/3894b8vqvoBfzczs4+Vyed9ae1dVdTgcPrn7p9bavK7rL621t621t+u6vn98fHzn7n9VVbXWqqrq5eVl1vs7/IkQM7O2bdu3aZreV1Wdz+e/3f1da+2r9/5tmqYP7n6qqvL5fK7WWrn7uapqnufq/bvhA5iZtcvl8r619q6q6nA4fHL3T621eV3XX1prb1tr1Vr7fr1e37TWvlZVTdNUVVWXy2XW+zv8CTAza9u2fZum6X1V1fl8/tvd37XWvnrvZmZVVTVNk7v7qffuvX8f/gSYmdn1ev3aWvtSVXU8Ht3d/zSzv6uqDofDVFV1PB6rqmqe5/J+qKrq+Xk4E2Bm1q7X69fW2peqquPx6O7+p5n9XVV1OBymqqrj8VhVVfM8l/dDVVU9Pw9nAszM2vV6/dpa+1JVdTwe3d3/NLO/q6oOh8NUVfXy8lLuXlVVz8/DmQAzs3a9Xr+21r5UVR2PR3f3P83s76qqw+EwVVU9Pz+Xu1dV1fPzcCbAzKxdr9eP0zR9rqo6Ho/u7sfWWlVVHQ6Hqarq+fm53L2qqp6fhzMBZmZtWZaP8zx/rqo6Ho/u7sfWWlVVHQ6Hqarq+fm53L2qqpaFEWBm1pZl+TjP8+eqquPx6O5+bK1VVdXhcJiqqpaFEWBm1pZl+TjP8+eqquPx6O5+bK1VVdWyMALMzNqyLB/nef5cVXU8Ht3dj621qqpaFkYAAAAAAAAAAAAAAAAAAAAAAAAAAADw//gHQgJYp6Pz3YAAAAAASUVORK5CYII=";

fn create_font_texture(context: &WebGlRenderingContext) -> Result<WebGlTexture, JsValue> {
    let texture = context.create_texture().unwrap();
    context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&texture));

    // Create an HTML Image element to load the font
    let image = web_sys::HtmlImageElement::new()?;
    
    // Set up image loading callback
    let callback = Closure::wrap(Box::new(move || {
        context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&texture));
        context.tex_image_2d_with_u32_and_u32_and_html_image_element(
            WebGlRenderingContext::TEXTURE_2D,
            0,
            WebGlRenderingContext::RGBA as i32,
            WebGlRenderingContext::RGBA,
            WebGlRenderingContext::UNSIGNED_BYTE,
            &image,
        ).unwrap();
        
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
    }) as Box<dyn FnMut()>);

    image.set_onload(Some(callback.as_ref().unchecked_ref()));
    image.set_src(TOPAZ_FONT_BITMAP);
    callback.forget();

    Ok(texture)
} 