// Copyright 2022 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Load rendering shaders.

#[cfg(feature = "wgpu")]
use wgpu::Device;

use crate::ShaderId;

#[cfg(feature = "wgpu")]
use crate::{
    Error, RendererOptions,
    recording::{BindType, ImageFormat},
    wgpu_engine::WgpuEngine,
};

// Shaders for the full pipeline
pub struct FullShaders {
    pub pathtag_reduce: ShaderId,
    pub pathtag_reduce2: ShaderId,
    pub pathtag_scan1: ShaderId,
    pub pathtag_scan: ShaderId,
    pub pathtag_scan_large: ShaderId,
    pub bbox_clear: ShaderId,
    pub flatten: ShaderId,
    pub draw_reduce: ShaderId,
    pub draw_leaf: ShaderId,
    pub clip_reduce: ShaderId,
    pub clip_leaf: ShaderId,
    pub binning: ShaderId,
    pub tile_alloc: ShaderId,
    pub backdrop: ShaderId,
    pub path_count_setup: ShaderId,
    pub path_count: ShaderId,
    pub coarse: ShaderId,
    pub path_tiling_setup: ShaderId,
    pub path_tiling: ShaderId,
    pub fine_area: Option<ShaderId>,
    pub fine_msaa8: Option<ShaderId>,
    pub fine_msaa16: Option<ShaderId>,
}

#[cfg(feature = "wgpu")]
pub(crate) fn full_shaders(
    device: &Device,
    engine: &mut WgpuEngine,
    options: &RendererOptions,
) -> Result<FullShaders, Error> {
    use BindType::*;

    let shaders = vello_shaders::SHADERS;

    macro_rules! add_shader {
        ($name:ident, $label:expr, $bindings:expr) => {{
            let source = shaders.$name.wgsl.code;
            engine.add_compute_shader(device, concat!("vello.", $label), source, &$bindings)
        }};
        ($name:ident, $bindings:expr) => {{ add_shader!($name, stringify!($name), $bindings) }};
    }

    let pathtag_reduce = add_shader!(pathtag_reduce, [Uniform, BufReadOnly, Buffer]);
    let pathtag_reduce2 = add_shader!(pathtag_reduce2, [BufReadOnly, Buffer]);
    let pathtag_scan1 = add_shader!(pathtag_scan1, [BufReadOnly, BufReadOnly, Buffer]);
    let pathtag_scan = add_shader!(
        pathtag_scan_small,
        [Uniform, BufReadOnly, BufReadOnly, Buffer]
    );
    let pathtag_scan_large = add_shader!(
        pathtag_scan_large,
        [Uniform, BufReadOnly, BufReadOnly, Buffer]
    );
    let bbox_clear = add_shader!(bbox_clear, [Uniform, Buffer]);
    let flatten = add_shader!(
        flatten,
        [Uniform, BufReadOnly, BufReadOnly, Buffer, Buffer, Buffer]
    );
    let draw_reduce = add_shader!(draw_reduce, [Uniform, BufReadOnly, Buffer]);
    let draw_leaf = add_shader!(
        draw_leaf,
        [
            Uniform,
            BufReadOnly,
            BufReadOnly,
            BufReadOnly,
            Buffer,
            Buffer,
            Buffer,
        ]
    );
    let clip_reduce = add_shader!(clip_reduce, [BufReadOnly, BufReadOnly, Buffer, Buffer]);
    let clip_leaf = add_shader!(
        clip_leaf,
        [
            Uniform,
            BufReadOnly,
            BufReadOnly,
            BufReadOnly,
            BufReadOnly,
            Buffer,
            Buffer,
        ]
    );
    let binning = add_shader!(
        binning,
        [
            Uniform,
            BufReadOnly,
            BufReadOnly,
            BufReadOnly,
            Buffer,
            Buffer,
            Buffer,
            Buffer,
        ]
    );
    let tile_alloc = add_shader!(
        tile_alloc,
        [Uniform, BufReadOnly, BufReadOnly, Buffer, Buffer, Buffer]
    );
    let path_count_setup = add_shader!(path_count_setup, [Buffer, Buffer]);
    let path_count = add_shader!(
        path_count,
        [Uniform, Buffer, BufReadOnly, BufReadOnly, Buffer, Buffer]
    );
    let backdrop = add_shader!(backdrop_dyn, [Uniform, Buffer, BufReadOnly, Buffer]);
    let coarse = add_shader!(
        coarse,
        [
            Uniform,
            BufReadOnly,
            BufReadOnly,
            BufReadOnly,
            BufReadOnly,
            BufReadOnly,
            Buffer,
            Buffer,
            Buffer,
        ]
    );
    let path_tiling_setup = add_shader!(path_tiling_setup, [Buffer, Buffer, Buffer]);
    let path_tiling = add_shader!(
        path_tiling,
        [
            Buffer,
            BufReadOnly,
            BufReadOnly,
            BufReadOnly,
            BufReadOnly,
            Buffer,
        ]
    );
    let fine_resources = [
        Uniform,
        BufReadOnly,
        BufReadOnly,
        BufReadOnly,
        Buffer,
        Image(ImageFormat::Rgba8),
        ImageRead(ImageFormat::Rgba8),
        // Mask LUT buffer, used only when MSAA is enabled.
        BufReadOnly,
    ];

    let aa_support = &options.antialiasing_support;
    let fine_area = if aa_support.area {
        Some(add_shader!(
            fine_area,
            fine_resources[..fine_resources.len() - 1]
        ))
    } else {
        None
    };
    let fine_msaa8 = if aa_support.msaa8 {
        Some(add_shader!(fine_msaa8, fine_resources))
    } else {
        None
    };
    let fine_msaa16 = if aa_support.msaa16 {
        Some(add_shader!(fine_msaa16, fine_resources))
    } else {
        None
    };

    Ok(FullShaders {
        pathtag_reduce,
        pathtag_reduce2,
        pathtag_scan,
        pathtag_scan1,
        pathtag_scan_large,
        bbox_clear,
        flatten,
        draw_reduce,
        draw_leaf,
        clip_reduce,
        clip_leaf,
        binning,
        tile_alloc,
        path_count_setup,
        path_count,
        backdrop,
        coarse,
        path_tiling_setup,
        path_tiling,
        fine_area,
        fine_msaa8,
        fine_msaa16,
    })
}
