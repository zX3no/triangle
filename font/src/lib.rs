#![feature(const_maybe_uninit_zeroed)]
use glow::*;
use std::mem::MaybeUninit;

extern crate nalgebra_glm as glm;

pub mod glyph;
pub use glyph::*;

pub static mut GL: MaybeUninit<Context> = MaybeUninit::uninit();

pub const TOP_LEFT: Vec2 = Vec2::new(-0.5, 0.5);
pub const BOTTOM_LEFT: Vec2 = Vec2::new(-0.5, -0.5);
pub const TOP_RIGHT: Vec2 = Vec2::new(0.5, 0.5);
pub const BOTTOM_RIGHT: Vec2 = Vec2::new(0.5, -0.5);
pub const UV_TOP_LEFT: Vec2 = Vec2::new(0.0, 1.0);
pub const UV_BOTTOM_LEFT: Vec2 = Vec2::new(0.0, 0.0);
pub const UV_TOP_RIGHT: Vec2 = Vec2::new(1.0, 1.0);
pub const UV_BOTTOM_RIGHT: Vec2 = Vec2::new(1.0, 0.0);

#[track_caller]
pub fn check_error(gl: &Context) {
    let error = unsafe { gl.get_error() };
    match error {
        glow::INVALID_ENUM => panic!("INVALID_ENUM"),
        glow::INVALID_VALUE => panic!("INVALID_VALUE"),
        glow::INVALID_OPERATION => panic!("INVALID_OPERATION"),
        glow::STACK_OVERFLOW => panic!("STACK_OVERFLOW"),
        glow::STACK_UNDERFLOW => panic!("STACK_UNDERFLOW"),
        glow::OUT_OF_MEMORY => panic!("OUT_OF_MEMORY"),
        glow::INVALID_FRAMEBUFFER_OPERATION => panic!("INVALID_FRAMEBUFFER_OPERATION"),
        0 => {}
        _ => unreachable!(),
    }
}

//TODO: Uniforms?
/// Macro for creating shaders.
/// ```rs
/// let program = shader! {
///     include_str!("../shaders/simple.vert"),
///     include_str!("../shaders/text.frag"),
///     Vec2 => 0,
///     Vec4 => 1,
///     Vec2 => 2
/// };
/// ```
#[macro_export]
macro_rules! shader {
    ($vert:expr, $frag:expr, $($type:ident => $position:expr),*$(,)?) => {
        unsafe {
            use glow::HasContext;

            let gl = $crate::GL.assume_init_ref();
            let mut stride = 0;
            let mut offset = 0;

            $(
                stride += std::mem::size_of::<$type>() ;
            )*

            $(
                let n = std::mem::size_of::<$type>() / std::mem::size_of::<f32>();
                gl.enable_vertex_attrib_array($position);
                gl.vertex_attrib_pointer_f32($position, n as i32, glow::FLOAT, false, stride as i32, offset as i32);
                offset += std::mem::size_of::<$type>();
            )*

            let program = gl.create_program().unwrap();

            let v = gl.create_shader(glow::VERTEX_SHADER).unwrap();
            let error = gl.get_shader_info_log(v);
            if !error.is_empty() {
                panic!("{error}");
            }
            gl.shader_source(v, $vert);
            gl.compile_shader(v);
            gl.attach_shader(program, v);

            let f = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
            let error = gl.get_shader_info_log(f);
            if !error.is_empty() {
                panic!("{error}");
            }
            gl.shader_source(f, $frag);
            gl.compile_shader(f);
            gl.attach_shader(program, f);

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            gl.delete_shader(v);
            gl.delete_shader(f);

            program
        }
   };
}

pub unsafe fn texture() -> NativeTexture {
    let gl = GL.assume_init_ref();
    let bytes = include_bytes!("../container.jpg");
    let im = image::load_from_memory(bytes).unwrap();
    let texture = gl.create_texture().unwrap();

    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MIN_FILTER,
        glow::LINEAR as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MAG_FILTER,
        glow::LINEAR as i32,
    );
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGB as i32,
        im.width() as i32,
        im.height() as i32,
        0,
        glow::RGB,
        glow::UNSIGNED_BYTE,
        Some(im.as_bytes()),
    );
    texture
}

//I think rust packed my struct in a weird way.
//So align won't work unless you use `repr(C)`.
#[repr(C)]
pub struct Vertex {
    pub position: Vec2,
    pub color: Vec4,
    pub uv: Vec2,
}

#[inline]
pub fn buffer(vertices: &[f32]) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts(
            vertices.as_ptr() as *const u8,
            vertices.len() * core::mem::size_of::<f32>(),
        )
    }
}

pub struct Renderer {
    pub gl: &'static glow::Context,
    pub vertices: Vec<Vertex>,
    pub vao: NativeVertexArray,
    pub vbo: NativeBuffer,
    pub buffer_size: usize,
}

impl Renderer {
    pub fn new(gl: &'static glow::Context) -> Self {
        unsafe {
            // gl.enable(glow::DEPTH_TEST);
            gl.enable(glow::DEBUG_OUTPUT);
            gl.enable(glow::DEBUG_OUTPUT_SYNCHRONOUS);
            gl.debug_message_callback(|source, ty, id, severity, msg| {
                if id == 131169 || id == 131185 || id == 131218 || id == 131204 {
                    return;
                }

                println!("---------------");
                println!("Debug message ({}): {}", id, msg);

                match source {
                    glow::DEBUG_SOURCE_API => println!("Source: API"),
                    glow::DEBUG_SOURCE_WINDOW_SYSTEM => println!("Source: Window System"),
                    glow::DEBUG_SOURCE_SHADER_COMPILER => println!("Source: Shader Compiler"),
                    glow::DEBUG_SOURCE_THIRD_PARTY => println!("Source: Third Party"),
                    glow::DEBUG_SOURCE_APPLICATION => println!("Source: Application"),
                    glow::DEBUG_SOURCE_OTHER => println!("Source: Other"),
                    _ => println!("Source: Unknown"),
                }

                match ty {
                    glow::DEBUG_TYPE_ERROR => println!("Type: Error"),
                    glow::DEBUG_TYPE_DEPRECATED_BEHAVIOR => println!("Type: Deprecated Behaviour"),
                    glow::DEBUG_TYPE_UNDEFINED_BEHAVIOR => println!("Type: Undefined Behaviour"),
                    glow::DEBUG_TYPE_PORTABILITY => println!("Type: Portability"),
                    glow::DEBUG_TYPE_PERFORMANCE => println!("Type: Performance"),
                    glow::DEBUG_TYPE_MARKER => println!("Type: Marker"),
                    glow::DEBUG_TYPE_PUSH_GROUP => println!("Type: Push Group"),
                    glow::DEBUG_TYPE_POP_GROUP => println!("Type: Pop Group"),
                    glow::DEBUG_TYPE_OTHER => println!("Type: Other"),
                    _ => println!("Type: Unknown"),
                }

                match severity {
                    glow::DEBUG_SEVERITY_HIGH => println!("Severity: High"),
                    glow::DEBUG_SEVERITY_MEDIUM => println!("Severity: Medium"),
                    glow::DEBUG_SEVERITY_LOW => println!("Severity: Low"),
                    glow::DEBUG_SEVERITY_NOTIFICATION => println!("Severity: Notification"),
                    _ => println!("Severity: Unknown"),
                }

                println!();
            });

            gl.clear_color(0.2, 0.2, 0.2, 0.2);

            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

            Self {
                gl,
                vao,
                vbo,
                vertices: Vec::new(),
                buffer_size: 0,
            }
        }
    }

    pub fn vertex(&mut self, position: Vec2, color: Vec4, uv: Vec2) {
        self.vertices.push(Vertex {
            position,
            color,
            uv,
        });
    }

    pub fn line(&mut self, p0: Vec2, p1: Vec2, color: Vec4) {
        self.vertex(p0, color, Vec2::new(0., 0.));
        self.vertex(p1, color, Vec2::new(0., 0.));
    }

    pub fn triangle(
        &mut self,
        p0: Vec2,
        p1: Vec2,
        p2: Vec2,
        c0: Vec4,
        c1: Vec4,
        c2: Vec4,
        uv0: Vec2,
        uv1: Vec2,
        uv2: Vec2,
    ) {
        self.vertex(p0, c0, uv0);
        self.vertex(p1, c1, uv1);
        self.vertex(p2, c2, uv2);
    }

    pub fn quad(
        &mut self,
        tr: Vec2,
        tl: Vec2,
        bl: Vec2,
        br: Vec2,
        c0: Vec4,
        c1: Vec4,
        c2: Vec4,
        c3: Vec4,
        uv0: Vec2,
        uv1: Vec2,
        uv2: Vec2,
        uv3: Vec2,
    ) {
        self.triangle(tr, tl, bl, c0, c1, c2, uv0, uv1, uv2);
        self.triangle(bl, br, tr, c1, c2, c3, uv1, uv2, uv3);
    }

    pub fn texture(&mut self, pos: Vec2, size: Vec2, uvp: Vec2, uvs: Vec2, color: Vec4) {
        self.quad(
            pos,
            pos + Vec2::new(size.x, 0.0),
            pos + Vec2::new(0.0, size.y),
            pos + size,
            color,
            color,
            color,
            color,
            uvp,
            uvp + Vec2::new(uvs.x, 0.0),
            uvp + Vec2::new(0.0, uvs.y),
            uvp + uvs,
        );
    }

    pub fn use_shader(&self, program: NativeProgram) {
        unsafe { self.gl.use_program(Some(program)) };
    }

    pub fn clear(&self) {
        unsafe {
            self.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }
    }

    pub fn draw(&mut self) {
        unsafe {
            //When replacing the entire data store, consider using glBufferSubData rather than completely recreating the data store with glBufferData. This avoids the cost of reallocating the data store.
            if self.buffer_size != self.vertices.len() {
                self.gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    self.vertices.align_to::<u8>().1,
                    glow::DYNAMIC_DRAW,
                );
                self.buffer_size = self.vertices.len();
            } else {
                self.gl.buffer_sub_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    0,
                    self.vertices.align_to::<u8>().1,
                );
            }

            // self.gl.draw_arrays(glow::LINES, 0, 2);
            self.gl
                .draw_arrays(glow::TRIANGLES, 0, self.vertices.len() as i32);
        }
    }

    pub fn update(&self, w: i32, h: i32) {
        unsafe { self.gl.viewport(0, 0, w, h) }
    }
}
