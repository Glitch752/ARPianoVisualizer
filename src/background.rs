// Inspired heavily by https://github.com/foxzool/bevy_nokhwa, but with a simpler shader that avoids a vertex/index buffer.

use bevy::asset::RenderAssetUsages;
use bevy::image::TextureFormatPixelInfo;
use bevy::{core_pipeline, prelude::*};
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_graph::{Node, RenderGraph, RenderLabel, RenderSubGraph};
use bevy::render::render_graph::{NodeRunError, RenderGraphContext, SlotInfo};
use bevy::render::render_resource::{
    AddressMode, BindGroup, BindGroupEntries, BindGroupLayoutEntry, BindingType, BlendComponent, BlendState, ColorTargetState, ColorWrites, Extent3d, Face, FilterMode, FrontFace, MultisampleState, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, RawFragmentState, RawRenderPipelineDescriptor, RawVertexState, RenderPassDescriptor, RenderPipeline, SamplerBindingType, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, TexelCopyBufferLayout, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue};
use bevy::render::view::{ExtractedView, ViewTarget};
use bevy::render::RenderApp;
use opencv::core::{AlgorithmHint, Mat, MatTraitConst, MatTraitConstManual};
use opencv::imgproc;

use crate::video::WebcamFrame;
use crate::VideoDrawSystems;

#[derive(Resource, Default)]
pub struct ConvertedWebcamFrame(pub Mat);

#[derive(Deref, DerefMut, Default, Resource, ExtractResource, Clone)]
pub struct BackgroundImage(pub Image);

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
pub struct BackgroundGraph;
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub(crate) struct BackgroundNodeLabel;

#[derive(Resource)]
pub struct BackgroundPipeline {
    render_pipeline: RenderPipeline,
}

impl FromWorld for BackgroundPipeline {
    fn from_world(world: &mut World) -> Self {
        let mut query = world.query_filtered::<&Msaa, With<Camera>>();
        let msaa = match query.single(world) {
            Ok(m) => *m,
            Err(_) => Msaa::Sample4,
        };
        let device = world.resource::<RenderDevice>();

        let shader = device.create_and_validate_shader_module(ShaderModuleDescriptor {
            label: Some("Webcam Shader"),
            source: ShaderSource::Wgsl(include_str!("backgroundShader.wgsl").into()),
        });

        let texture_bind_group_layout = device.create_bind_group_layout(
            "webcam_bind_group_layout",
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        );

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Webcam Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RawRenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: RawVertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(RawFragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState {
                        color: BlendComponent::REPLACE,
                        alpha: BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: msaa.samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            cache: None,
        });

        Self { render_pipeline }
    }
}

pub struct BackgroundPassDriverNode;

impl Node for BackgroundPassDriverNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        graph.run_sub_graph(BackgroundGraph, vec![], Some(graph.view_entity()))?;

        Ok(())
    }
}

pub struct BackgroundNode {
    query: QueryState<&'static ViewTarget, With<ExtractedView>>,
    diffuse_bind_group: Option<BindGroup>,
}

impl BackgroundNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
            diffuse_bind_group: None,
        }
    }
}

impl Node for BackgroundNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
        if let Some(img) = world.get_resource::<BackgroundImage>() {
            let device = world.get_resource::<RenderDevice>().unwrap();
            let queue = world.get_resource::<RenderQueue>().unwrap();

            let size = Extent3d {
                width: img.width(),
                height: img.height(),
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(&TextureDescriptor {
                label: Some("webcam_img"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let format_size = img.texture_descriptor.format.pixel_size();
            queue.write_texture(
                texture.as_image_copy(),
                img.data.as_ref().expect("Image has no data"),
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(img.width() * format_size as u32),
                    rows_per_image: None,
                },
                img.texture_descriptor.size,
            );

            let view = texture.create_view(&TextureViewDescriptor::default());
            let sampler = device.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Nearest,
                mipmap_filter: FilterMode::Nearest,
                ..Default::default()
            });

            let texture_bind_group_layout = device.create_bind_group_layout(
                "texture_bind_group_layout",
                &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            );

            let diffuse_bind_group = device.create_bind_group(
                Some("diffuse_bind_group"),
                &texture_bind_group_layout,
                &BindGroupEntries::sequential((&view, &sampler)),
            );

            self.diffuse_bind_group = Some(diffuse_bind_group);
        }
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        for target in self.query.iter_manual(world) {
            let pipeline = world.get_resource::<BackgroundPipeline>().unwrap();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("background_pass"),
                color_attachments: &[Some(target.get_color_attachment())],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            };

            let mut render_pass = render_context
                .command_encoder()
                .begin_render_pass(&pass_descriptor);

            render_pass.set_pipeline(&pipeline.render_pipeline);

            render_pass.set_bind_group(0, self.diffuse_bind_group.as_ref().unwrap(), &[]);
            render_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}

pub fn handle_background_image(
    mut image: ResMut<BackgroundImage>,
    mut webcam_frame: ResMut<WebcamFrame>,
    mut converted_webcam_frame: ResMut<ConvertedWebcamFrame>
) {
    // Retrieve the latest frame from the webcam
    let frame = &mut webcam_frame.0;
    let converted_frame = &mut converted_webcam_frame.0;

    if imgproc::cvt_color(frame, converted_frame, imgproc::COLOR_BGR2RGBA, 0, AlgorithmHint::ALGO_HINT_DEFAULT).is_err() {
        eprintln!("Failed to convert frame to RGBA format");
        return;
    }

    // Get image dimensions
    let (width, height) = (converted_frame.cols() as u32, converted_frame.rows() as u32);

    // Get the image data
    let data = match converted_frame.data_bytes() {
        Ok(data) => data.to_vec(),
        Err(_) => {
            eprintln!("Failed to get image data from frame");
            return;
        }
    };

    let size = Extent3d {
        width, height,
        depth_or_array_layers: 1,
    };
    let dimensions = TextureDimension::D2;
    let format = TextureFormat::Rgba8Unorm;
    let asset_usage = RenderAssetUsages::default();
    image.0 = Image::new(size, dimensions, data, format, asset_usage);
}


pub struct CameraBackground;

impl Plugin for CameraBackground {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ClearColor(Color::NONE))
            .insert_resource(BackgroundImage(Image::default()))
            .insert_resource(ConvertedWebcamFrame(Mat::default()))
            .add_plugins(ExtractResourcePlugin::<BackgroundImage>::default())
            .add_systems(Update, handle_background_image.in_set(VideoDrawSystems));

        let render_app = app.sub_app_mut(RenderApp);

        let background_node_2d = BackgroundNode::new(render_app.world_mut());
        let background_node_3d = BackgroundNode::new(render_app.world_mut());
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();

        if let Some(graph_2d) = render_graph.get_sub_graph_mut(core_pipeline::core_2d::graph::Core2d) {
            graph_2d.add_node(BackgroundNodeLabel, background_node_2d);

            graph_2d.add_node_edge(
                BackgroundNodeLabel,
                core_pipeline::core_2d::graph::Node2d::StartMainPass,
            );
        }

        if let Some(graph_3d) = render_graph.get_sub_graph_mut(core_pipeline::core_3d::graph::Core3d) {
            graph_3d.add_node(BackgroundNodeLabel, background_node_3d);

            graph_3d.add_node_edge(
                BackgroundNodeLabel,
                core_pipeline::core_3d::graph::Node3d::MainTransparentPass,
            );
        }
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<BackgroundPipeline>();
    }
}