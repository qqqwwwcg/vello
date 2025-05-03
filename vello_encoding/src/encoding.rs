// Copyright 2022 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::{DrawColor, DrawImage, DrawTag, Patch, PathEncoder, PathTag, Style, Transform};

use peniko::kurbo::{Shape, Stroke};
use peniko::{BlendMode, BrushRef, Fill, Image};

/// Encoded data streams for a scene.
///
/// # Invariants
///
/// * At least one transform and style must be encoded before any path data
///   or draw object.
#[derive(Clone, Default)]
pub struct Encoding {
    /// The path tag stream.
    pub path_tags: Vec<PathTag>,
    /// The path data stream.
    /// Stores all coordinates on paths.
    /// Stored as `u32` as all comparisons are performed bitwise.
    pub path_data: Vec<u32>,
    /// The draw tag stream.
    pub draw_tags: Vec<DrawTag>,
    /// The draw data stream.
    pub draw_data: Vec<u32>,
    /// The transform stream.
    pub transforms: Vec<Transform>,
    /// The style stream
    pub styles: Vec<Style>,
    /// Late bound resource data.
    pub resources: Resources,
    /// Number of encoded paths.
    pub n_paths: u32,
    /// Number of encoded path segments.
    pub n_path_segments: u32,
    /// Number of encoded clips/layers.
    pub n_clips: u32,
    /// Number of unclosed clips/layers.
    pub n_open_clips: u32,
    /// Flags that capture the current state of the encoding.
    pub flags: u32,
}

impl Encoding {
    /// Forces encoding of the next transform even if it matches
    /// the current transform in the stream.
    pub const FORCE_NEXT_TRANSFORM: u32 = 1;

    /// Forces encoding of the next style even if it matches
    /// the current style in the stream.
    pub const FORCE_NEXT_STYLE: u32 = 2;

    /// Creates a new encoding.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the encoding is empty.
    pub fn is_empty(&self) -> bool {
        self.path_tags.is_empty()
    }

    #[doc(alias = "clear")]
    // This is not called "clear" because "clear" has other implications
    // in graphics contexts.
    /// Clears the encoding.
    pub fn reset(&mut self) {
        self.transforms.clear();
        self.path_tags.clear();
        self.path_data.clear();
        self.styles.clear();
        self.draw_data.clear();
        self.draw_tags.clear();
        self.n_paths = 0;
        self.n_path_segments = 0;
        self.n_clips = 0;
        self.n_open_clips = 0;
        self.flags = 0;
        self.resources.reset();
    }

    /// Appends another encoding to this one with an optional transform.
    pub fn append(&mut self, other: &Self, transform: &Option<Transform>) {
        {
            self.resources
                .patches
                .extend(other.resources.patches.clone());
        };
        self.path_tags.extend_from_slice(&other.path_tags);
        self.path_data.extend_from_slice(&other.path_data);
        self.draw_tags.extend_from_slice(&other.draw_tags);
        self.draw_data.extend_from_slice(&other.draw_data);
        self.n_paths += other.n_paths;
        self.n_path_segments += other.n_path_segments;
        self.n_clips += other.n_clips;
        self.n_open_clips += other.n_open_clips;
        self.flags = other.flags;
        if let Some(transform) = *transform {
            self.transforms
                .extend(other.transforms.iter().map(|x| transform * *x));
        } else {
            self.transforms.extend_from_slice(&other.transforms);
        }
        self.styles.extend_from_slice(&other.styles);
    }

    /// Encodes a fill style.
    pub fn encode_fill_style(&mut self, fill: Fill) {
        self.encode_style(Style::from_fill(fill));
    }

    /// Encodes a stroke style.
    pub fn encode_stroke_style(&mut self, stroke: &Stroke) {
        self.encode_style(Style::from_stroke(stroke));
    }

    fn encode_style(&mut self, style: Style) {
        if self.flags & Self::FORCE_NEXT_STYLE != 0 || self.styles.last() != Some(&style) {
            self.path_tags.push(PathTag::STYLE);
            self.styles.push(style);
            self.flags &= !Self::FORCE_NEXT_STYLE;
        }
    }

    /// Encodes a transform.
    ///
    /// If the given transform is different from the current one, encodes it and
    /// returns true. Otherwise, encodes nothing and returns false.
    pub fn encode_transform(&mut self, transform: Transform) -> bool {
        if self.flags & Self::FORCE_NEXT_TRANSFORM != 0
            || self.transforms.last() != Some(&transform)
        {
            self.path_tags.push(PathTag::TRANSFORM);
            self.transforms.push(transform);
            self.flags &= !Self::FORCE_NEXT_TRANSFORM;
            true
        } else {
            false
        }
    }

    /// Returns an encoder for encoding a path. If `is_fill` is true, all subpaths will
    /// be automatically closed.
    pub fn encode_path(&mut self, is_fill: bool) -> PathEncoder<'_> {
        PathEncoder::new(
            &mut self.path_tags,
            &mut self.path_data,
            &mut self.n_path_segments,
            &mut self.n_paths,
            is_fill,
        )
    }

    /// Encodes a shape. If `is_fill` is true, all subpaths will be automatically closed.
    /// Returns `true` if a non-zero number of segments were encoded.
    pub fn encode_shape(&mut self, shape: &impl Shape, is_fill: bool) -> bool {
        let mut encoder = self.encode_path(is_fill);
        encoder.shape(shape);
        encoder.finish(true) != 0
    }

    /// Encode an empty path.
    ///
    /// This is useful for bookkeeping when a path is absolutely required (for example in
    /// pushing a clip layer). It is almost always the case, however, that an application
    /// can be optimized to not use this method.
    pub fn encode_empty_shape(&mut self) {
        let mut encoder = self.encode_path(true);
        encoder.empty_path();
        encoder.finish(true);
    }

    /// Encodes a path element iterator. If `is_fill` is true, all subpaths will be automatically
    /// closed. Returns `true` if a non-zero number of segments were encoded.
    pub fn encode_path_elements(
        &mut self,
        path: impl Iterator<Item = peniko::kurbo::PathEl>,
        is_fill: bool,
    ) -> bool {
        let mut encoder = self.encode_path(is_fill);
        encoder.path_elements(path);
        encoder.finish(true) != 0
    }

    /// Encodes a brush with an optional alpha modifier.
    #[expect(
        single_use_lifetimes,
        reason = "False positive: https://github.com/rust-lang/rust/issues/129255"
    )]
    pub fn encode_brush<'b>(&mut self, brush: impl Into<BrushRef<'b>>, alpha: f32) {
        match brush.into() {
            BrushRef::Solid(color) => {
                let color = if alpha != 1.0 {
                    color.multiply_alpha(alpha)
                } else {
                    color
                };
                self.encode_color(color);
            }
            BrushRef::Gradient(_) => {}
            BrushRef::Image(image) => {
                self.encode_image(image, alpha);
            }
        }
    }

    /// Encodes a solid color brush.
    pub fn encode_color(&mut self, color: impl Into<DrawColor>) {
        let color = color.into();
        self.draw_tags.push(DrawTag::COLOR);
        let DrawColor { rgba } = color;
        self.draw_data.push(rgba);
    }

    /// Encodes an image brush.
    pub fn encode_image(&mut self, image: &Image, alpha: f32) {
        let alpha = (alpha * image.alpha * 255.0).round() as u8;
        // TODO: feed the alpha multiplier through the full pipeline for consistency
        // with other brushes?
        // Tracked in https://github.com/linebender/vello/issues/692
        self.resources.patches.push(Patch {
            image: image.clone(),
            draw_data_offset: self.draw_data.len(),
        });
        self.draw_tags.push(DrawTag::IMAGE);
        self.draw_data
            .extend_from_slice(bytemuck::cast_slice(bytemuck::bytes_of(&DrawImage {
                xy: 0,
                width_height: (image.width << 16) | (image.height & 0xFFFF),
                sample_alpha: ((image.quality as u32) << 12)
                    | ((image.x_extend as u32) << 10)
                    | ((image.y_extend as u32) << 8)
                    | alpha as u32,
            })));
    }

    /// Encodes a begin clip command.
    pub fn encode_begin_clip(&mut self, blend_mode: BlendMode, alpha: f32) {
        use super::DrawBeginClip;
        self.draw_tags.push(DrawTag::BEGIN_CLIP);
        self.draw_data
            .extend_from_slice(bytemuck::cast_slice(bytemuck::bytes_of(
                &DrawBeginClip::new(blend_mode, alpha),
            )));
        self.n_clips += 1;
        self.n_open_clips += 1;
    }

    /// Encodes an end clip command.
    pub fn encode_end_clip(&mut self) {
        if self.n_open_clips > 0 {
            self.draw_tags.push(DrawTag::END_CLIP);
            // This is a dummy path, and will go away with the new clip impl.
            self.path_tags.push(PathTag::PATH);
            self.n_paths += 1;
            self.n_clips += 1;
            self.n_open_clips -= 1;
        }
    }

    /// Forces the next transform and style to be encoded even if they match
    /// the current state.
    pub fn force_next_transform_and_style(&mut self) {
        self.flags |= Self::FORCE_NEXT_TRANSFORM | Self::FORCE_NEXT_STYLE;
    }

    // Swap the last two tags in the path tag stream; used for transformed
    // gradients.
    pub fn swap_last_path_tags(&mut self) {
        let len = self.path_tags.len();
        self.path_tags.swap(len - 1, len - 2);
    }
}

/// Encoded data for late bound resources.
#[derive(Clone, Default)]
pub struct Resources {
    /// Draw data patches for late bound resources.
    pub patches: Vec<Patch>,
}

impl Resources {
    #[doc(alias = "clear")]
    // This is not called "clear" because "clear" has other implications
    // in graphics contexts.
    fn reset(&mut self) {
        self.patches.clear();
    }
}
