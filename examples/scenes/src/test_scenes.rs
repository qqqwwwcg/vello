// Copyright 2022 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::{ExampleScene, SceneConfig, SceneSet};
use vello::{
    kurbo::{Affine, Cap},
    peniko::ImageQuality,
};

/// All of the test scenes supported by Vello.
pub fn test_scenes() -> SceneSet {
    test_scenes_inner()
}

/// A macro which exports each passed scene indivudally
///
/// This is used to avoid having to repeatedly define a
macro_rules! export_scenes {
    ($($scene_name: ident($($scene: tt)+)),*$(,)?) => {
        pub fn test_scenes_inner() -> SceneSet {
            let scenes = vec![
                $($scene_name()),+
            ];
            SceneSet { scenes }
        }

        $(
            pub fn $scene_name() -> ExampleScene {
                scene!($($scene)+)
            }
        )+
    };
}

/// A helper to create a shorthand name for a single scene.
/// Used in `export_scenes`.
macro_rules! scene {
    ($name: ident) => {
        scene!($name: false)
    };
    ($name: ident: animated) => {
        scene!($name: true)
    };
    ($name: ident: $animated: literal) => {
        scene!(impls::$name, stringify!($name), $animated)
    };
    ($func:expr, $name: expr, $animated: literal) => {
        ExampleScene {
            config: SceneConfig {
                animated: $animated,
                name: $name.to_owned(),
            },
            function: Box::new($func),
        }
    };
}

export_scenes!(
    stroke_styles(impls::stroke_styles(Affine::IDENTITY), "stroke_styles", false),
    stroke_styles_non_uniform(impls::stroke_styles(Affine::scale_non_uniform(1.2, 0.7)), "stroke_styles (non-uniform scale)", false),
    stroke_styles_skew(impls::stroke_styles(Affine::skew(1., 0.)), "stroke_styles (skew)", false),
    fill_types(fill_types),
    cardioid_and_friends(cardioid_and_friends),
    two_point_radial(two_point_radial),
    brush_transform(brush_transform: animated),
    blend_grid(blend_grid),
    deep_blend(deep_blend),
    many_clips(many_clips),
    conflation_artifacts(conflation_artifacts),
    labyrinth(labyrinth),
    robust_paths(robust_paths),
    base_color_test(base_color_test: animated),
    clip_test(clip_test: animated),
    longpathdash_butt(impls::longpathdash(Cap::Butt), "longpathdash (butt caps)", false),
    longpathdash_round(impls::longpathdash(Cap::Round), "longpathdash (round caps)", false),
    mmark(crate::mmark::MMark::new(80_000), "mmark", false),
    many_draw_objects(many_draw_objects),
    image_sampling(image_sampling),
    image_extend_modes_bilinear(impls::image_extend_modes(ImageQuality::Medium), "image_extend_modes (bilinear)", false),
    image_extend_modes_nearest_neighbor(impls::image_extend_modes(ImageQuality::Low), "image_extend_modes (nearest neighbor)", false),
);

/// Implementations for the test scenes.
/// In a module because the exported [`ExampleScene`] creation functions use the same names.
mod impls {
    use std::f64::consts::{FRAC_1_SQRT_2, PI};
    use std::sync::Arc;

    use crate::SceneParams;
    use rand::Rng;
    use rand::{SeedableRng, rngs::StdRng};
    use vello::kurbo::{
        Affine, BezPath, Cap, Circle, Ellipse, Join, PathEl, Point, Rect, Shape, Stroke, Vec2,
    };
    use vello::peniko::color::{AlphaColor, Lch, palette};
    use vello::peniko::*;
    use vello::*;

    const FLOWER_IMAGE: &[u8] = include_bytes!("../../assets/splash-flower.jpg");

    pub(super) fn stroke_styles(transform: Affine) -> impl FnMut(&mut Scene, &mut SceneParams<'_>) {
        use PathEl::*;
        move |scene, params| {
            let colors = [
                Color::from_rgb8(140, 181, 236),
                Color::from_rgb8(246, 236, 202),
                Color::from_rgb8(201, 147, 206),
                Color::from_rgb8(150, 195, 160),
            ];
            let simple_stroke = [MoveTo((0., 0.).into()), LineTo((100., 0.).into())];
            let join_stroke = [
                MoveTo((0., 0.).into()),
                CurveTo((20., 0.).into(), (42.5, 5.).into(), (50., 25.).into()),
                CurveTo((57.5, 5.).into(), (80., 0.).into(), (100., 0.).into()),
            ];
            let miter_stroke = [
                MoveTo((0., 0.).into()),
                LineTo((90., 16.).into()),
                LineTo((0., 31.).into()),
                LineTo((90., 46.).into()),
            ];
            let closed_strokes = [
                MoveTo((0., 0.).into()),
                LineTo((90., 21.).into()),
                LineTo((0., 42.).into()),
                ClosePath,
                MoveTo((200., 0.).into()),
                CurveTo((100., 72.).into(), (300., 72.).into(), (200., 0.).into()),
                ClosePath,
                MoveTo((290., 0.).into()),
                CurveTo((200., 72.).into(), (400., 72.).into(), (310., 0.).into()),
                ClosePath,
            ];
            let cap_styles = [Cap::Butt, Cap::Square, Cap::Round];
            let join_styles = [Join::Bevel, Join::Miter, Join::Round];
            let miter_limits = [4., 6., 0.1, 10.];

            // Simple strokes with cap combinations
            let t = Affine::translate((60., 40.)) * Affine::scale(2.);
            let mut y = 0.;
            let mut color_idx = 0;
            for start in cap_styles {
                for end in cap_styles {
                    scene.stroke(
                        &Stroke::new(20.).with_start_cap(start).with_end_cap(end),
                        Affine::translate((0., y + 30.)) * t * transform,
                        colors[color_idx],
                        None,
                        &simple_stroke,
                    );
                    y += 180.;
                    color_idx = (color_idx + 1) % colors.len();
                }
            }
            // Dashed strokes with cap combinations
            let t = Affine::translate((450., 0.)) * t;
            let mut y_max = y;
            y = 0.;
            for start in cap_styles {
                for end in cap_styles {
                    scene.stroke(
                        &Stroke::new(20.)
                            .with_start_cap(start)
                            .with_end_cap(end)
                            .with_dashes(0., [10.0, 21.0]),
                        Affine::translate((0., y + 30.)) * t * transform,
                        colors[color_idx],
                        None,
                        &simple_stroke,
                    );
                    y += 180.;
                    color_idx = (color_idx + 1) % colors.len();
                }
            }

            // Cap and join combinations
            let t = Affine::translate((550., 0.)) * t;
            y_max = y_max.max(y);
            y = 0.;
            for cap in cap_styles {
                for join in join_styles {
                    scene.stroke(
                        &Stroke::new(20.).with_caps(cap).with_join(join),
                        Affine::translate((0., y + 30.)) * t * transform,
                        colors[color_idx],
                        None,
                        &join_stroke,
                    );
                    y += 185.;
                    color_idx = (color_idx + 1) % colors.len();
                }
            }

            // Miter limit
            let t = Affine::translate((500., 0.)) * t;
            y_max = y_max.max(y);
            y = 0.;
            for ml in miter_limits {
                scene.stroke(
                    &Stroke::new(10.)
                        .with_caps(Cap::Butt)
                        .with_join(Join::Miter)
                        .with_miter_limit(ml),
                    Affine::translate((0., y + 30.)) * t * transform,
                    colors[color_idx],
                    None,
                    &miter_stroke,
                );
                y += 180.;
                color_idx = (color_idx + 1) % colors.len();
            }

            // Closed paths
            for (i, join) in join_styles.iter().enumerate() {
                // The cap style is not important since a closed path shouldn't have any caps.
                scene.stroke(
                    &Stroke::new(10.)
                        .with_caps(cap_styles[i])
                        .with_join(*join)
                        .with_miter_limit(5.),
                    Affine::translate((0., y + 30.)) * t * transform,
                    colors[color_idx],
                    None,
                    &closed_strokes,
                );
                y += 180.;
                color_idx = (color_idx + 1) % colors.len();
            }
            y_max = y_max.max(y);
            // The closed_strokes has a maximum x of 400, `t` has a scale of `2.`
            // Give 50px of padding to account for `transform`
            let x_max = t.translation().x + 400. * 2. + 50.;
            params.resolution = Some((x_max, y_max).into());
        }
    }

    pub(super) fn fill_types(scene: &mut Scene, params: &mut SceneParams<'_>) {
        use PathEl::*;
        params.resolution = Some((1400., 700.).into());
        let rect = Rect::from_origin_size(Point::new(0., 0.), (500., 500.));
        let star = [
            MoveTo((250., 0.).into()),
            LineTo((105., 450.).into()),
            LineTo((490., 175.).into()),
            LineTo((10., 175.).into()),
            LineTo((395., 450.).into()),
            ClosePath,
        ];
        let arcs = [
            MoveTo((0., 480.).into()),
            CurveTo((500., 480.).into(), (500., -10.).into(), (0., -10.).into()),
            ClosePath,
            MoveTo((500., -10.).into()),
            CurveTo((0., -10.).into(), (0., 480.).into(), (500., 480.).into()),
            ClosePath,
        ];
        let scale = Affine::scale(0.6);
        let t = Affine::translate((10., 25.));
        let rules = [
            (Fill::NonZero, "Non-Zero", star.as_slice()),
            (Fill::EvenOdd, "Even-Odd", &star),
            (Fill::NonZero, "Non-Zero", &arcs),
            (Fill::EvenOdd, "Even-Odd", &arcs),
        ];
        for (i, rule) in rules.iter().enumerate() {
            let t = Affine::translate(((i % 2) as f64 * 306., (i / 2) as f64 * 340.)) * t;
            let t = Affine::translate((0., 5.)) * t * scale;
            scene.fill(Fill::NonZero, t, palette::css::GRAY, None, &rect);
            scene.fill(
                rule.0,
                Affine::translate((0., 10.)) * t,
                palette::css::YELLOW,
                None,
                &rule.2,
            );
        }

        // Draw blends
        let t = Affine::translate((700., 0.)) * t;
        for (i, rule) in rules.iter().enumerate() {
            let t = Affine::translate(((i % 2) as f64 * 306., (i / 2) as f64 * 340.)) * t;
            let t = Affine::translate((0., 5.)) * t * scale;
            scene.fill(Fill::NonZero, t, palette::css::GRAY, None, &rect);
            scene.fill(
                rule.0,
                Affine::translate((0., 10.)) * t,
                palette::css::YELLOW,
                None,
                &rule.2,
            );
            scene.fill(
                rule.0,
                Affine::translate((0., 10.)) * t * Affine::rotate(0.06),
                Color::new([0., 1., 0.7, 0.6]),
                None,
                &rule.2,
            );
            scene.fill(
                rule.0,
                Affine::translate((0., 10.)) * t * Affine::rotate(-0.06),
                Color::new([0.9, 0.7, 0.5, 0.6]),
                None,
                &rule.2,
            );
        }
    }

    pub(super) fn cardioid_and_friends(scene: &mut Scene, _: &mut SceneParams<'_>) {
        render_cardioid(scene);
        render_clip_test(scene);
        render_alpha_test(scene);
        //render_tiger(scene, false);
    }

    pub(super) fn longpathdash(cap: Cap) -> impl FnMut(&mut Scene, &mut SceneParams<'_>) {
        use PathEl::*;
        move |scene, _| {
            let mut path = BezPath::new();
            let mut x = 32;
            while x < 256 {
                let mut a: f64 = 0.0;
                while a < PI * 2.0 {
                    let pts = [
                        (256.0 + a.sin() * x as f64, 256.0 + a.cos() * x as f64),
                        (
                            256.0 + (a + PI / 3.0).sin() * (x + 64) as f64,
                            256.0 + (a + PI / 3.0).cos() * (x + 64) as f64,
                        ),
                    ];
                    path.push(MoveTo(pts[0].into()));
                    let mut i: f64 = 0.0;
                    while i < 1.0 {
                        path.push(LineTo(
                            (
                                pts[0].0 * (1.0 - i) + pts[1].0 * i,
                                pts[0].1 * (1.0 - i) + pts[1].1 * i,
                            )
                                .into(),
                        ));
                        i += 0.05;
                    }
                    a += PI * 0.01;
                }
                x += 16;
            }
            scene.stroke(
                &Stroke::new(1.0)
                    .with_caps(cap)
                    .with_join(Join::Bevel)
                    .with_dashes(0.0, [1.0, 1.0]),
                Affine::translate((50.0, 50.0)),
                palette::css::YELLOW,
                None,
                &path,
            );
        }
    }

    pub(super) fn brush_transform(scene: &mut Scene, params: &mut SceneParams<'_>) {
        let th = params.time;
        let linear = Gradient::new_linear((0.0, 0.0), (0.0, 200.0)).with_stops([
            palette::css::RED,
            palette::css::GREEN,
            palette::css::BLUE,
        ]);
        scene.fill(
            Fill::NonZero,
            Affine::rotate(25_f64.to_radians()) * Affine::scale_non_uniform(2.0, 1.0),
            &Gradient::new_radial((200.0, 200.0), 80.0).with_stops([
                palette::css::RED,
                palette::css::GREEN,
                palette::css::BLUE,
            ]),
            None,
            &Rect::from_origin_size((100.0, 100.0), (200.0, 200.0)),
        );
        scene.fill(
            Fill::NonZero,
            Affine::translate((200.0, 600.0)),
            &linear,
            Some(around_center(Affine::rotate(th), Point::new(200.0, 100.0))),
            &Rect::from_origin_size(Point::default(), (400.0, 200.0)),
        );
        scene.stroke(
            &Stroke::new(40.0),
            Affine::translate((800.0, 600.0)),
            &linear,
            Some(around_center(Affine::rotate(th), Point::new(200.0, 100.0))),
            &Rect::from_origin_size(Point::default(), (400.0, 200.0)),
        );
    }

    pub(super) fn two_point_radial(scene: &mut Scene, _params: &mut SceneParams<'_>) {
        pub(super) fn make(
            scene: &mut Scene,
            x0: f64,
            y0: f64,
            r0: f32,
            x1: f64,
            y1: f64,
            r1: f32,
            transform: Affine,
            extend: Extend,
        ) {
            let colors = [
                palette::css::RED,
                palette::css::YELLOW,
                Color::from_rgb8(6, 85, 186),
            ];
            let width = 400_f64;
            let height = 200_f64;
            let rect = Rect::new(0.0, 0.0, width, height);
            scene.fill(Fill::NonZero, transform, palette::css::WHITE, None, &rect);
            scene.fill(
                Fill::NonZero,
                transform,
                &Gradient::new_two_point_radial((x0, y0), r0, (x1, y1), r1)
                    .with_stops(colors)
                    .with_extend(extend),
                None,
                &Rect::new(0.0, 0.0, width, height),
            );
            let r0 = r0 as f64 - 1.0;
            let r1 = r1 as f64 - 1.0;
            let stroke_width = 1.0;
            scene.stroke(
                &Stroke::new(stroke_width),
                transform,
                palette::css::BLACK,
                None,
                &Ellipse::new((x0, y0), (r0, r0), 0.0),
            );
            scene.stroke(
                &Stroke::new(stroke_width),
                transform,
                palette::css::BLACK,
                None,
                &Ellipse::new((x1, y1), (r1, r1), 0.0),
            );
        }

        // These demonstrate radial gradient patterns similar to the examples shown
        // at <https://learn.microsoft.com/en-us/typography/opentype/spec/colr#radial-gradients>

        for (i, mode) in [Extend::Pad, Extend::Repeat, Extend::Reflect]
            .iter()
            .enumerate()
        {
            let y = 100.0;
            let x0 = 140.0;
            let x1 = x0 + 140.0;
            let r0 = 20.0;
            let r1 = 50.0;
            make(
                scene,
                x0,
                y,
                r0,
                x1,
                y,
                r1,
                Affine::translate((i as f64 * 420.0 + 20.0, 20.0)),
                *mode,
            );
        }

        for (i, mode) in [Extend::Pad, Extend::Repeat, Extend::Reflect]
            .iter()
            .enumerate()
        {
            let y = 100.0;
            let x0 = 140.0;
            let x1 = x0 + 140.0;
            let r0 = 20.0;
            let r1 = 50.0;
            make(
                scene,
                x1,
                y,
                r1,
                x0,
                y,
                r0,
                Affine::translate((i as f64 * 420.0 + 20.0, 240.0)),
                *mode,
            );
        }

        for (i, mode) in [Extend::Pad, Extend::Repeat, Extend::Reflect]
            .iter()
            .enumerate()
        {
            let y = 100.0;
            let x0 = 140.0;
            let x1 = x0 + 140.0;
            let r0 = 50.0;
            let r1 = 50.0;
            make(
                scene,
                x0,
                y,
                r0,
                x1,
                y,
                r1,
                Affine::translate((i as f64 * 420.0 + 20.0, 460.0)),
                *mode,
            );
        }

        for (i, mode) in [Extend::Pad, Extend::Repeat, Extend::Reflect]
            .iter()
            .enumerate()
        {
            let x0 = 140.0;
            let y0 = 125.0;
            let r0 = 20.0;
            let x1 = 190.0;
            let y1 = 100.0;
            let r1 = 95.0;
            make(
                scene,
                x0,
                y0,
                r0,
                x1,
                y1,
                r1,
                Affine::translate((i as f64 * 420.0 + 20.0, 680.0)),
                *mode,
            );
        }

        for (i, mode) in [Extend::Pad, Extend::Repeat, Extend::Reflect]
            .iter()
            .enumerate()
        {
            let x0 = 140.0;
            let y0 = 125.0;
            let r0 = 20.0;
            let x1 = 190.0;
            let y1 = 100.0;
            let r1 = 96.0;
            // Shift p0 so the outer edges of both circles touch
            let p0 = Point::new(x1, y1)
                + ((Point::new(x0, y0) - Point::new(x1, y1)).normalize() * (r1 - r0));
            make(
                scene,
                p0.x,
                p0.y,
                r0 as f32,
                x1,
                y1,
                r1 as f32,
                Affine::translate((i as f64 * 420.0 + 20.0, 900.0)),
                *mode,
            );
        }
    }

    pub(super) fn blend_grid(scene: &mut Scene, _: &mut SceneParams<'_>) {
        const BLEND_MODES: &[Mix] = &[
            Mix::Normal,
            Mix::Multiply,
            Mix::Darken,
            Mix::Screen,
            Mix::Lighten,
            Mix::Overlay,
            Mix::ColorDodge,
            Mix::ColorBurn,
            Mix::HardLight,
            Mix::SoftLight,
            Mix::Difference,
            Mix::Exclusion,
            Mix::Hue,
            Mix::Saturation,
            Mix::Color,
            Mix::Luminosity,
        ];
        for (ix, &blend) in BLEND_MODES.iter().enumerate() {
            let i = ix % 4;
            let j = ix / 4;
            let transform = Affine::translate((i as f64 * 225., j as f64 * 225.));
            let square = blend_square(blend.into());
            scene.append(&square, Some(transform));
        }
    }

    pub(super) fn deep_blend(scene: &mut Scene, params: &mut SceneParams<'_>) {
        params.resolution = Some(Vec2::new(1000., 1000.));
        let main_rect = Rect::from_origin_size((10., 10.), (900., 900.));
        scene.fill(
            Fill::EvenOdd,
            Affine::IDENTITY,
            palette::css::RED,
            None,
            &main_rect,
        );
        let options = [
            (800., palette::css::AQUA),
            (700., palette::css::RED),
            (600., palette::css::ALICE_BLUE),
            (500., palette::css::YELLOW),
            (400., palette::css::GREEN),
            (300., palette::css::BLUE),
            (200., palette::css::ORANGE),
            (100., palette::css::WHITE),
        ];
        let mut depth = 0;
        for (width, color) in &options[..params.complexity.min(options.len() - 1)] {
            scene.push_layer(
                Mix::Normal,
                0.9,
                Affine::IDENTITY,
                &Rect::from_origin_size((10., 10.), (*width, *width)),
            );
            scene.fill(Fill::EvenOdd, Affine::IDENTITY, color, None, &main_rect);
            depth += 1;
        }
        for _ in 0..depth {
            scene.pop_layer();
        }
    }

    pub(super) fn many_clips(scene: &mut Scene, params: &mut SceneParams<'_>) {
        params.resolution = Some(Vec2::new(1000., 1000.));
        let mut rng = StdRng::seed_from_u64(42);
        let mut base_tri = BezPath::new();
        base_tri.move_to((-50.0, 0.0));
        base_tri.line_to((25.0, -43.3));
        base_tri.line_to((25.0, 43.3));
        for y in 0..10 {
            for x in 0..10 {
                let translate =
                    Affine::translate((100. * (x as f64 + 0.5), 100. * (y as f64 + 0.5)));
                const CLIPS_PER_FILL: usize = 3;
                for _ in 0..CLIPS_PER_FILL {
                    let rot = Affine::rotate(rng.random_range(0.0..PI));
                    scene.push_layer(Mix::Clip, 1.0, translate * rot, &base_tri);
                }
                let rot = Affine::rotate(rng.random_range(0.0..PI));
                let color = Color::new([rng.random(), rng.random(), rng.random(), 1.]);
                scene.fill(Fill::NonZero, translate * rot, color, None, &base_tri);
                for _ in 0..CLIPS_PER_FILL {
                    scene.pop_layer();
                }
            }
        }
    }

    // Support functions

    pub(super) fn render_cardioid(scene: &mut Scene) {
        let n = 601;
        let dth = PI * 2.0 / (n as f64);
        let center = Point::new(1024.0, 768.0);
        let r = 750.0;
        let mut path = BezPath::new();
        for i in 1..n {
            let mut p0 = center;
            let a0 = i as f64 * dth;
            p0.x += a0.cos() * r;
            p0.y += a0.sin() * r;
            let mut p1 = center;
            let a1 = ((i * 2) % n) as f64 * dth;
            p1.x += a1.cos() * r;
            p1.y += a1.sin() * r;
            path.push(PathEl::MoveTo(p0));
            path.push(PathEl::LineTo(p1));
        }
        scene.stroke(
            &Stroke::new(2.0),
            Affine::IDENTITY,
            palette::css::BLUE,
            None,
            &path,
        );
    }

    pub(super) fn render_clip_test(scene: &mut Scene) {
        const N: usize = 16;
        const X0: f64 = 50.0;
        const Y0: f64 = 450.0;
        // Note: if it gets much larger, it will exceed the 1MB scratch buffer.
        // But this is a pretty demanding test.
        const X1: f64 = 550.0;
        const Y1: f64 = 950.0;
        let step = 1.0 / ((N + 1) as f64);
        for i in 0..N {
            let t = ((i + 1) as f64) * step;
            let path = [
                PathEl::MoveTo((X0, Y0).into()),
                PathEl::LineTo((X1, Y0).into()),
                PathEl::LineTo((X1, Y0 + t * (Y1 - Y0)).into()),
                PathEl::LineTo((X1 + t * (X0 - X1), Y1).into()),
                PathEl::LineTo((X0, Y1).into()),
                PathEl::ClosePath,
            ];
            scene.push_layer(Mix::Clip, 1.0, Affine::IDENTITY, &path);
        }
        let rect = Rect::new(X0, Y0, X1, Y1);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            palette::css::LIME,
            None,
            &rect,
        );
        for _ in 0..N {
            scene.pop_layer();
        }
    }

    pub(super) fn render_alpha_test(scene: &mut Scene) {
        // Alpha compositing tests.
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            palette::css::RED,
            None,
            &make_diamond(1024.0, 100.0),
        );
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            palette::css::LIME.with_alpha(0.5),
            None,
            &make_diamond(1024.0, 125.0),
        );
        scene.push_layer(
            Mix::Clip,
            1.0,
            Affine::IDENTITY,
            &make_diamond(1024.0, 150.0),
        );
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            palette::css::BLUE.with_alpha(0.5),
            None,
            &make_diamond(1024.0, 175.0),
        );
        scene.pop_layer();
    }

    pub(super) fn render_blend_square(scene: &mut Scene, blend: BlendMode, transform: Affine) {
        // Inspired by https://developer.mozilla.org/en-US/docs/Web/CSS/mix-blend-mode
        let rect = Rect::from_origin_size(Point::new(0., 0.), (200., 200.));
        let linear = Gradient::new_linear((0.0, 0.0), (200.0, 0.0))
            .with_stops([palette::css::BLACK, palette::css::WHITE]);
        scene.fill(Fill::NonZero, transform, &linear, None, &rect);
        const GRADIENTS: &[(f64, f64, Color)] = &[
            (150., 0., Color::from_rgb8(255, 240, 64)),
            (175., 100., Color::from_rgb8(255, 96, 240)),
            (125., 200., Color::from_rgb8(64, 192, 255)),
        ];
        for (x, y, c) in GRADIENTS {
            let color2 = c.with_alpha(0.);
            let radial = Gradient::new_radial((*x, *y), 100.0).with_stops([*c, color2]);
            scene.fill(Fill::NonZero, transform, &radial, None, &rect);
        }
        const COLORS: &[Color] = &[palette::css::RED, palette::css::LIME, palette::css::BLUE];
        scene.push_layer(Mix::Normal, 1.0, transform, &rect);
        for (i, c) in COLORS.iter().enumerate() {
            let linear = Gradient::new_linear((0.0, 0.0), (0.0, 200.0))
                .with_stops([palette::css::WHITE, *c]);
            scene.push_layer(blend, 1.0, transform, &rect);
            // squash the ellipse
            let a = transform
                * Affine::translate((100., 100.))
                * Affine::rotate(std::f64::consts::FRAC_PI_3 * (i * 2 + 1) as f64)
                * Affine::scale_non_uniform(1.0, 0.357)
                * Affine::translate((-100., -100.));
            scene.fill(
                Fill::NonZero,
                a,
                &linear,
                None,
                &Ellipse::new((100., 100.), (90., 90.), 0.),
            );
            scene.pop_layer();
        }
        scene.pop_layer();
    }

    pub(super) fn blend_square(blend: BlendMode) -> Scene {
        let mut fragment = Scene::default();
        render_blend_square(&mut fragment, blend, Affine::IDENTITY);
        fragment
    }

    pub(super) fn conflation_artifacts(scene: &mut Scene, _: &mut SceneParams<'_>) {
        use PathEl::*;
        const N: f64 = 50.0;
        const S: f64 = 4.0;

        let scale = Affine::scale(S);
        let x = N + 0.5; // Fractional pixel offset reveals the problem on axis-aligned edges.
        let mut y = N;

        let bg_color = Color::from_rgb8(255, 194, 19);
        let fg_color = Color::from_rgb8(12, 165, 255);

        // Two adjacent triangles touching at diagonal edge with opposing winding numbers
        scene.fill(
            Fill::NonZero,
            Affine::translate((x, y)) * scale,
            fg_color,
            None,
            &[
                // triangle 1
                MoveTo((0.0, 0.0).into()),
                LineTo((N, N).into()),
                LineTo((0.0, N).into()),
                LineTo((0.0, 0.0).into()),
                // triangle 2
                MoveTo((0.0, 0.0).into()),
                LineTo((N, N).into()),
                LineTo((N, 0.0).into()),
                LineTo((0.0, 0.0).into()),
            ],
        );

        // Adjacent rects, opposite winding
        y += S * N + 10.0;
        scene.fill(
            Fill::EvenOdd,
            Affine::translate((x, y)) * scale,
            bg_color,
            None,
            &Rect::new(0.0, 0.0, N, N),
        );
        scene.fill(
            Fill::EvenOdd,
            Affine::translate((x, y)) * scale,
            fg_color,
            None,
            &[
                // left rect
                MoveTo((0.0, 0.0).into()),
                LineTo((0.0, N).into()),
                LineTo((N * 0.5, N).into()),
                LineTo((N * 0.5, 0.0).into()),
                // right rect
                MoveTo((N * 0.5, 0.0).into()),
                LineTo((N, 0.0).into()),
                LineTo((N, N).into()),
                LineTo((N * 0.5, N).into()),
            ],
        );

        // Adjacent rects, same winding
        y += S * N + 10.0;
        scene.fill(
            Fill::EvenOdd,
            Affine::translate((x, y)) * scale,
            bg_color,
            None,
            &Rect::new(0.0, 0.0, N, N),
        );
        scene.fill(
            Fill::EvenOdd,
            Affine::translate((x, y)) * scale,
            fg_color,
            None,
            &[
                // left rect
                MoveTo((0.0, 0.0).into()),
                LineTo((0.0, N).into()),
                LineTo((N * 0.5, N).into()),
                LineTo((N * 0.5, 0.0).into()),
                // right rect
                MoveTo((N * 0.5, 0.0).into()),
                LineTo((N * 0.5, N).into()),
                LineTo((N, N).into()),
                LineTo((N, 0.0).into()),
            ],
        );
    }

    pub(super) fn labyrinth(scene: &mut Scene, _: &mut SceneParams<'_>) {
        use PathEl::*;

        let rows: &[[u8; 12]] = &[
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            [0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1, 1],
            [0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1],
            [1, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0],
            [0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1],
            [1, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 0],
            [0, 1, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0],
            [1, 0, 1, 0, 1, 1, 1, 1, 0, 1, 1, 1],
            [0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1],
            [0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0],
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        ];
        let cols: &[[u8; 10]] = &[
            [1, 1, 1, 1, 0, 1, 1, 1, 1, 1],
            [0, 0, 1, 0, 0, 0, 1, 1, 1, 0],
            [0, 1, 1, 0, 1, 1, 1, 0, 0, 1],
            [1, 1, 0, 0, 0, 0, 1, 0, 1, 0],
            [0, 0, 1, 0, 1, 0, 0, 0, 0, 1],
            [0, 0, 1, 1, 1, 0, 0, 0, 1, 0],
            [0, 1, 0, 1, 1, 1, 0, 0, 0, 0],
            [1, 1, 1, 0, 1, 1, 1, 0, 1, 0],
            [1, 1, 0, 1, 1, 0, 0, 0, 1, 0],
            [0, 0, 1, 0, 0, 0, 0, 0, 0, 1],
            [0, 0, 1, 1, 0, 0, 0, 0, 1, 0],
            [0, 0, 0, 0, 0, 0, 1, 0, 0, 1],
            [1, 1, 1, 1, 1, 1, 0, 1, 1, 1],
        ];
        let mut path = BezPath::new();
        for (y, row) in rows.iter().enumerate() {
            for (x, flag) in row.iter().enumerate() {
                let x = x as f64;
                let y = y as f64;
                if *flag == 1 {
                    path.push(MoveTo((x - 0.1, y + 0.1).into()));
                    path.push(LineTo((x + 1.1, y + 0.1).into()));
                    path.push(LineTo((x + 1.1, y - 0.1).into()));
                    path.push(LineTo((x - 0.1, y - 0.1).into()));

                    // The above is equivalent to the following stroke with width 0.2 and square
                    // caps.
                    //path.push(MoveTo((x, y).into()));
                    //path.push(LineTo((x + 1.0, y).into()));
                }
            }
        }
        for (x, col) in cols.iter().enumerate() {
            for (y, flag) in col.iter().enumerate() {
                let x = x as f64;
                let y = y as f64;
                if *flag == 1 {
                    path.push(MoveTo((x - 0.1, y - 0.1).into()));
                    path.push(LineTo((x - 0.1, y + 1.1).into()));
                    path.push(LineTo((x + 0.1, y + 1.1).into()));
                    path.push(LineTo((x + 0.1, y - 0.1).into()));
                    // The above is equivalent to the following stroke with width 0.2 and square
                    // caps.
                    //path.push(MoveTo((x, y).into()));
                    //path.push(LineTo((x, y + 1.0).into()));
                }
            }
        }

        // Note the artifacts are clearly visible at a fractional pixel offset/translation. They
        // disappear if the translation amount below is a whole number.
        scene.fill(
            Fill::NonZero,
            Affine::translate((20.5, 20.5)) * Affine::scale(80.0),
            Color::from_rgb8(0x70, 0x80, 0x80),
            None,
            &path,
        );
    }

    pub(super) fn robust_paths(scene: &mut Scene, _: &mut SceneParams<'_>) {
        let mut path = BezPath::new();
        path.move_to((16.0, 16.0));
        path.line_to((32.0, 16.0));
        path.line_to((32.0, 32.0));
        path.line_to((16.0, 32.0));
        path.close_path();
        path.move_to((48.0, 18.0));
        path.line_to((64.0, 23.0));
        path.line_to((64.0, 33.0));
        path.line_to((48.0, 38.0));
        path.close_path();
        path.move_to((80.0, 18.0));
        path.line_to((82.0, 16.0));
        path.line_to((94.0, 16.0));
        path.line_to((96.0, 18.0));
        path.line_to((96.0, 30.0));
        path.line_to((94.0, 32.0));
        path.line_to((82.0, 32.0));
        path.line_to((80.0, 30.0));
        path.close_path();
        path.move_to((112.0, 16.0));
        path.line_to((128.0, 16.0));
        path.line_to((128.0, 32.0));
        path.close_path();
        path.move_to((144.0, 16.0));
        path.line_to((160.0, 32.0));
        path.line_to((144.0, 32.0));
        path.close_path();
        path.move_to((168.0, 8.0));
        path.line_to((184.0, 8.0));
        path.line_to((184.0, 24.0));
        path.close_path();
        path.move_to((200.0, 8.0));
        path.line_to((216.0, 24.0));
        path.line_to((200.0, 24.0));
        path.close_path();
        path.move_to((241.0, 17.5));
        path.line_to((255.0, 17.5));
        path.line_to((255.0, 19.5));
        path.line_to((241.0, 19.5));
        path.close_path();
        path.move_to((241.0, 22.5));
        path.line_to((256.0, 22.5));
        path.line_to((256.0, 24.5));
        path.line_to((241.0, 24.5));
        path.close_path();
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            palette::css::YELLOW,
            None,
            &path,
        );
        scene.fill(
            Fill::EvenOdd,
            Affine::translate((300.0, 0.0)),
            palette::css::LIME,
            None,
            &path,
        );

        path.move_to((8.0, 4.0));
        path.line_to((8.0, 40.0));
        path.line_to((260.0, 40.0));
        path.line_to((260.0, 4.0));
        path.close_path();
        scene.fill(
            Fill::NonZero,
            Affine::translate((0.0, 100.0)),
            palette::css::YELLOW,
            None,
            &path,
        );
        scene.fill(
            Fill::EvenOdd,
            Affine::translate((300.0, 100.0)),
            palette::css::LIME,
            None,
            &path,
        );
    }

    pub(super) fn base_color_test(scene: &mut Scene, params: &mut SceneParams<'_>) {
        // Cycle through the hue value every 5 seconds (t % 5) * 360/5
        let color = AlphaColor::<Lch>::new([80., 80., (params.time % 5.) as f32 * 72., 1.]);
        params.base_color = Some(color.convert());

        // Blend a white square over it.
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            palette::css::WHITE.with_alpha(0.5),
            None,
            &Rect::new(50.0, 50.0, 500.0, 500.0),
        );
    }

    pub(super) fn clip_test(scene: &mut Scene, params: &mut SceneParams<'_>) {
        let clip = {
            const X0: f64 = 50.0;
            const Y0: f64 = 0.0;
            const X1: f64 = 200.0;
            const Y1: f64 = 500.0;
            [
                PathEl::MoveTo((X0, Y0).into()),
                PathEl::LineTo((X1, Y0).into()),
                PathEl::LineTo((X1, Y0 + (Y1 - Y0)).into()),
                PathEl::LineTo((X1 + (X0 - X1), Y1).into()),
                PathEl::LineTo((X0, Y1).into()),
                PathEl::ClosePath,
            ]
        };
        scene.push_layer(Mix::Clip, 1.0, Affine::IDENTITY, &clip);

        scene.pop_layer();

        let large_background_rect = Rect::new(-1000.0, -1000.0, 2000.0, 2000.0);
        let inside_clip_rect = Rect::new(11.0, 13.399999999999999, 59.0, 56.6);
        let outside_clip_rect = Rect::new(
            12.599999999999998,
            12.599999999999998,
            57.400000000000006,
            57.400000000000006,
        );
        let clip_rect = Rect::new(0.0, 0.0, 74.4, 339.20000000000005);
        let scale = 2.0;

        scene.push_layer(
            BlendMode {
                mix: Mix::Normal,
                compose: Compose::SrcOver,
            },
            1.0,
            Affine::new([scale, 0.0, 0.0, scale, 27.07470703125, 176.40660533027858]),
            &clip_rect,
        );

        scene.fill(
            Fill::NonZero,
            Affine::new([scale, 0.0, 0.0, scale, 27.07470703125, 176.40660533027858]),
            palette::css::BLUE,
            None,
            &large_background_rect,
        );
        scene.fill(
            Fill::NonZero,
            Affine::new([
                scale,
                0.0,
                0.0,
                scale,
                29.027636718750003,
                182.9755506427786,
            ]),
            palette::css::LIME,
            None,
            &inside_clip_rect,
        );
        scene.fill(
            Fill::NonZero,
            Affine::new([
                scale,
                0.0,
                0.0,
                scale,
                29.027636718750003,
                scale * 559.3583631427786,
            ]),
            palette::css::RED,
            None,
            &outside_clip_rect,
        );

        scene.pop_layer();
    }

    pub(super) fn around_center(xform: Affine, center: Point) -> Affine {
        Affine::translate(center.to_vec2()) * xform * Affine::translate(-center.to_vec2())
    }

    pub(super) fn make_diamond(cx: f64, cy: f64) -> [PathEl; 5] {
        const SIZE: f64 = 50.0;
        [
            PathEl::MoveTo(Point::new(cx, cy - SIZE)),
            PathEl::LineTo(Point::new(cx + SIZE, cy)),
            PathEl::LineTo(Point::new(cx, cy + SIZE)),
            PathEl::LineTo(Point::new(cx - SIZE, cy)),
            PathEl::ClosePath,
        ]
    }

    pub(super) fn many_draw_objects(scene: &mut Scene, params: &mut SceneParams<'_>) {
        const N_WIDE: usize = 300;
        const N_HIGH: usize = 300;
        const SCENE_WIDTH: f64 = 2000.0;
        const SCENE_HEIGHT: f64 = 1500.0;
        params.resolution = Some((SCENE_WIDTH, SCENE_HEIGHT).into());
        for j in 0..N_HIGH {
            let y = (j as f64 + 0.5) * (SCENE_HEIGHT / N_HIGH as f64);
            for i in 0..N_WIDE {
                let x = (i as f64 + 0.5) * (SCENE_WIDTH / N_WIDE as f64);
                let c = Circle::new((x, y), 3.0);
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    palette::css::YELLOW,
                    None,
                    &c,
                );
            }
        }
    }

    pub(super) fn image_sampling(scene: &mut Scene, params: &mut SceneParams<'_>) {
        params.resolution = Some(Vec2::new(1100., 1100.));
        params.base_color = Some(palette::css::WHITE);
        let mut blob: Vec<u8> = Vec::new();
        [
            palette::css::RED,
            palette::css::BLUE,
            palette::css::CYAN,
            palette::css::MAGENTA,
        ]
        .iter()
        .for_each(|c| {
            blob.extend(c.premultiply().to_rgba8().to_u8_array());
        });
        let data = Blob::new(Arc::new(blob));
        let image = Image::new(data, ImageFormat::Rgba8, 2, 2);

        scene.draw_image(
            &image,
            Affine::scale(200.).then_translate((100., 100.).into()),
        );
        scene.draw_image(
            &image,
            Affine::translate((-1., -1.))
                // 45° rotation
                .then_rotate(PI / 4.)
                .then_translate((1., 1.).into())
                // So the major axis is sqrt(2.) larger
                .then_scale(200. * FRAC_1_SQRT_2)
                .then_translate((100., 600.0).into()),
        );
        scene.draw_image(
            &image,
            Affine::scale_non_uniform(100., 200.).then_translate((600.0, 100.0).into()),
        );
        scene.draw_image(
            &image,
            Affine::skew(0.1, 0.25)
                .then_scale(200.0)
                .then_translate((600.0, 600.0).into()),
        );
    }

    pub(super) fn image_extend_modes(
        quality: ImageQuality,
    ) -> impl FnMut(&mut Scene, &mut SceneParams<'_>) {
        move |scene, params| {
            params.resolution = Some(Vec2::new(1500., 1500.));
            params.base_color = Some(palette::css::WHITE);
            let mut blob: Vec<u8> = Vec::new();
            [
                palette::css::RED,
                palette::css::BLUE,
                palette::css::CYAN,
                palette::css::MAGENTA,
            ]
            .iter()
            .for_each(|c| {
                blob.extend(c.premultiply().to_rgba8().to_u8_array());
            });
            let data = Blob::new(Arc::new(blob));
            let image = Image::new(data, ImageFormat::Rgba8, 2, 2).with_quality(quality);
            let brush_offset = Some(Affine::translate((2., 2.)));
            // Pad extend mode
            let image = image.with_extend(Extend::Pad);
            scene.fill(
                Fill::NonZero,
                Affine::scale(100.).then_translate((100., 100.).into()),
                &image,
                brush_offset,
                &Rect::new(0., 0., 6., 6.),
            );
            let image = image.with_extend(Extend::Reflect);
            scene.fill(
                Fill::NonZero,
                Affine::scale(100.).then_translate((100., 800.).into()),
                &image,
                brush_offset,
                &Rect::new(0., 0., 6., 6.),
            );
            let image = image.with_extend(Extend::Repeat);
            scene.fill(
                Fill::NonZero,
                Affine::scale(100.).then_translate((800., 100.).into()),
                &image,
                brush_offset,
                &Rect::new(0., 0., 6., 6.),
            );
            let image = image
                .with_x_extend(Extend::Repeat)
                .with_y_extend(Extend::Reflect);
            scene.fill(
                Fill::NonZero,
                Affine::scale(100.).then_translate((800., 800.).into()),
                &image,
                brush_offset,
                &Rect::new(0., 0., 6., 6.),
            );
        }
    }
}
