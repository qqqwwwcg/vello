// Copyright 2022 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use peniko::{
    BlendMode, BrushRef, Fill, Image, Mix,
    kurbo::{Affine, Rect, Shape, Stroke, StrokeOpts},
};

use vello_encoding::{Encoding, Transform};

// TODO - Document invariants and edge cases (#470)
// - What happens when we pass a transform matrix with NaN values to the Scene?
// - What happens if a push_layer isn't matched by a pop_layer?

/// The main datatype for rendering graphics.
///
/// A `Scene` stores a sequence of drawing commands, their context, and the
/// associated resources, which can later be rendered.
///
/// Most users will render this using [`Renderer::render_to_texture`][crate::Renderer::render_to_texture].
///
/// Rendering from a `Scene` will *not* clear it, which should be done in a separate step, by calling [`Scene::reset`].
///
/// If this is not done for a scene which is retained (to avoid allocations) between frames, this will likely
/// quickly increase the complexity of the render result, leading to crashes or potential host system instability.
#[derive(Clone, Default)]
pub struct Scene {
    encoding: Encoding,
}
static_assertions::assert_impl_all!(Scene: Send, Sync);

impl Scene {
    /// Creates a new scene.
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes all content from the scene.
    pub fn reset(&mut self) {
        self.encoding.reset();
    }

    /// Returns the underlying raw encoding.
    pub fn encoding(&self) -> &Encoding {
        &self.encoding
    }

    /// Returns a mutable reference to the underlying raw encoding.
    ///
    /// This can be used to more easily create invalid scenes, and so should be used with care.
    pub fn encoding_mut(&mut self) -> &mut Encoding {
        &mut self.encoding
    }

    /// Pushes a new layer clipped by the specified shape and composed with
    /// previous layers using the specified blend mode.
    ///
    /// Every drawing command after this call will be clipped by the shape
    /// until the layer is popped.
    ///
    /// **However, the transforms are *not* saved or modified by the layer stack.**
    ///
    /// Clip layers (`blend` = [`Mix::Clip`]) should have an alpha value of 1.0.
    /// For an opacity group with non-unity alpha, specify [`Mix::Normal`].
    pub fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        clip: &impl Shape,
    ) {
        let blend = blend.into();
        if blend.mix == Mix::Clip && alpha != 1.0 {
            log::warn!("Clip mix mode used with semitransparent alpha");
        }
        let t = Transform::from_kurbo(&transform);
        self.encoding.encode_transform(t);
        self.encoding.encode_fill_style(Fill::NonZero);
        if !self.encoding.encode_shape(clip, true) {
            // If the layer shape is invalid, encode a valid empty path. This suppresses
            // all drawing until the layer is popped.
            self.encoding.encode_empty_shape();
        }
        self.encoding
            .encode_begin_clip(blend, alpha.clamp(0.0, 1.0));
    }

    /// Pops the current layer.
    pub fn pop_layer(&mut self) {
        self.encoding.encode_end_clip();
    }

    /// Fills a shape using the specified style and brush.
    #[expect(
        single_use_lifetimes,
        reason = "False positive: https://github.com/rust-lang/rust/issues/129255"
    )]
    pub fn fill<'b>(
        &mut self,
        style: Fill,
        transform: Affine,
        brush: impl Into<BrushRef<'b>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        let t = Transform::from_kurbo(&transform);
        self.encoding.encode_transform(t);
        self.encoding.encode_fill_style(style);
        if self.encoding.encode_shape(shape, true) {
            if let Some(brush_transform) = brush_transform {
                if self
                    .encoding
                    .encode_transform(Transform::from_kurbo(&(transform * brush_transform)))
                {
                    self.encoding.swap_last_path_tags();
                }
            }
            self.encoding.encode_brush(brush, 1.0);
        }
    }

    /// Strokes a shape using the specified style and brush.
    #[expect(
        single_use_lifetimes,
        reason = "False positive: https://github.com/rust-lang/rust/issues/129255"
    )]
    pub fn stroke<'b>(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: impl Into<BrushRef<'b>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        // The setting for tolerance are a compromise. For most applications,
        // shape tolerance doesn't matter, as the input is likely Bézier paths,
        // which is exact. Note that shape tolerance is hard-coded as 0.1 in
        // the encoding crate.
        //
        // Stroke tolerance is a different matter. Generally, the cost scales
        // with inverse O(n^6), so there is moderate rendering cost to setting
        // too fine a value. On the other hand, error scales with the transform
        // applied post-stroking, so may exceed visible threshold. When we do
        // GPU-side stroking, the transform will be known. In the meantime,
        // this is a compromise.
        const SHAPE_TOLERANCE: f64 = 0.01;
        const STROKE_TOLERANCE: f64 = SHAPE_TOLERANCE;

        const GPU_STROKES: bool = true; // Set this to `true` to enable GPU-side stroking
        if GPU_STROKES {
            let t = Transform::from_kurbo(&transform);
            self.encoding.encode_transform(t);
            self.encoding.encode_stroke_style(style);

            // We currently don't support dashing on the GPU. If the style has a dash pattern, then
            // we convert it into stroked paths on the CPU and encode those as individual draw
            // objects.
            let encode_result = if style.dash_pattern.is_empty() {
                self.encoding.encode_shape(shape, false)
            } else {
                // TODO: We currently collect the output of the dash iterator because
                // `encode_path_elements` wants to consume the iterator. We want to avoid calling
                // `dash` twice when `bump_estimate` is enabled because it internally allocates.
                // Bump estimation will move to resolve time rather than scene construction time,
                // so we can revert this back to not collecting when that happens.
                let dashed = peniko::kurbo::dash(
                    shape.path_elements(SHAPE_TOLERANCE),
                    style.dash_offset,
                    &style.dash_pattern,
                )
                .collect::<Vec<_>>();

                self.encoding
                    .encode_path_elements(dashed.into_iter(), false)
            };
            if encode_result {
                if let Some(brush_transform) = brush_transform {
                    if self
                        .encoding
                        .encode_transform(Transform::from_kurbo(&(transform * brush_transform)))
                    {
                        self.encoding.swap_last_path_tags();
                    }
                }
                self.encoding.encode_brush(brush, 1.0);
            }
        } else {
            let stroked = peniko::kurbo::stroke(
                shape.path_elements(SHAPE_TOLERANCE),
                style,
                &StrokeOpts::default(),
                STROKE_TOLERANCE,
            );
            self.fill(Fill::NonZero, transform, brush, brush_transform, &stroked);
        }
    }

    /// Draws an image at its natural size with the given transform.
    pub fn draw_image(&mut self, image: &Image, transform: Affine) {
        self.fill(
            Fill::NonZero,
            transform,
            image,
            None,
            &Rect::new(0.0, 0.0, image.width as f64, image.height as f64),
        );
    }

    /// Appends a child scene.
    ///
    /// The given transform is applied to every transform in the child.
    /// This is an O(N) operation.
    pub fn append(&mut self, other: &Self, transform: Option<Affine>) {
        let t = transform.as_ref().map(Transform::from_kurbo);
        self.encoding.append(&other.encoding, &t);
    }
}

impl From<Encoding> for Scene {
    fn from(encoding: Encoding) -> Self {
        // It's fine to create a default estimator here, and that field will be
        // removed at some point - see https://github.com/linebender/vello/issues/541
        Self { encoding }
    }
}
