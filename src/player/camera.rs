use cgmath::{Point3, Vector3};
use wgpu::{BindGroupLayout, util::DeviceExt};
use crate::{bevy::prelude::*, position::FloatingPosition, render::wgpu_context::RenderDevice};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

#[derive(Component)]
pub struct Camera {
    pub eye: FloatingPosition,
    pub orientation: Vec3,
    pub up: Vec3,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self, aspect_ratio: f32) -> cgmath::Matrix4<f32> {
        let eye = self.eye.0;
        let eye = Point3::new(eye.x, eye.y, eye.z);
        let orientation = self.orientation;
        let orientation = Vector3::new(orientation.x, orientation.y, orientation.z);
        let up = self.up;
        let up = Vector3::new(up.x, up.y, up.z);

        let view = cgmath::Matrix4::look_to_rh(eye, orientation, up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), aspect_ratio, self.znear, self.zfar);
        OPENGL_TO_WGPU_MATRIX * proj * view
    }

    pub fn bake(&self, device: &RenderDevice, aspect_ratio: f32) -> BakedCamera {
        // We need this for Rust to store our data correctly for the shaders
        #[repr(C)]
        // This is so we can store this in a buffer
        #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct CameraUniform {
            // We can't use cgmath with bytemuck directly, so we'll have
            // to convert the Matrix4 into a 4x4 f32 array
            view_proj: [[f32; 4]; 4],
        }


        #[allow(clippy::cast_possible_truncation)]
        let uniform = CameraUniform {
            view_proj: self.build_view_projection_matrix(aspect_ratio).into(),
        };

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Self::bind_group_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        BakedCamera { bind_group }
    }

    pub fn bind_group_layout(device: &RenderDevice) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Camera Bind Group Layout"),
        })
    }
}

pub struct BakedCamera {
    pub bind_group: wgpu::BindGroup,
}