// Copyright 2022 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use bytemuck::{Pod, Zeroable};
use peniko::Image;

use crate::image_cache::{ImageCache, Images};

use super::{DrawTag, Encoding, PathTag, Style, Transform};

/// Layout of a packed encoding.
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
#[repr(C)]
pub struct Layout {
    /// Number of draw objects.
    pub n_draw_objects: u32,
    /// Number of paths.
    pub n_paths: u32,
    /// Number of clips.
    pub n_clips: u32,
    /// Start of binning data.
    pub bin_data_start: u32,
    /// Start of path tag stream.
    pub path_tag_base: u32,
    /// Start of path data stream.
    pub path_data_base: u32,
    /// Start of draw tag stream.
    pub draw_tag_base: u32,
    /// Start of draw data stream.
    pub draw_data_base: u32,
    /// Start of transform stream.
    pub transform_base: u32,
    /// Start of style stream.
    pub style_base: u32,
}

impl Layout {
    /// Creates a zeroed layout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the path tag stream.
    pub fn path_tags<'a>(&self, data: &'a [u8]) -> &'a [PathTag] {
        let start = self.path_tag_base as usize * 4;
        let end = self.path_data_base as usize * 4;
        bytemuck::cast_slice(&data[start..end])
    }

    pub fn path_tags_size(&self) -> u32 {
        let start = self.path_tag_base * 4;
        let end = self.path_data_base * 4;
        end - start
    }

    /// Returns the path tag stream in chunks of 4.
    pub fn path_tags_chunked<'a>(&self, data: &'a [u8]) -> &'a [u32] {
        let start = self.path_tag_base as usize * 4;
        let end = self.path_data_base as usize * 4;
        bytemuck::cast_slice(&data[start..end])
    }

    /// Returns the path data stream.
    pub fn path_data<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        let start = self.path_data_base as usize * 4;
        let end = self.draw_tag_base as usize * 4;
        bytemuck::cast_slice(&data[start..end])
    }

    /// Returns the draw tag stream.
    pub fn draw_tags<'a>(&self, data: &'a [u8]) -> &'a [DrawTag] {
        let start = self.draw_tag_base as usize * 4;
        let end = self.draw_data_base as usize * 4;
        bytemuck::cast_slice(&data[start..end])
    }

    /// Returns the draw data stream.
    pub fn draw_data<'a>(&self, data: &'a [u8]) -> &'a [u32] {
        let start = self.draw_data_base as usize * 4;
        let end = self.transform_base as usize * 4;
        bytemuck::cast_slice(&data[start..end])
    }

    /// Returns the transform stream.
    pub fn transforms<'a>(&self, data: &'a [u8]) -> &'a [Transform] {
        let start = self.transform_base as usize * 4;
        let end = self.style_base as usize * 4;
        bytemuck::cast_slice(&data[start..end])
    }

    /// Returns the style stream.
    pub fn styles<'a>(&self, data: &'a [u8]) -> &'a [Style] {
        let start = self.style_base as usize * 4;
        bytemuck::cast_slice(&data[start..])
    }
}

/// Resolves and packs an encoding that contains only paths with solid color
/// fills.
///
/// Panics if the encoding contains any late bound resources (gradients, images
/// or glyph runs).
pub fn resolve_solid_paths_only(encoding: &Encoding, packed: &mut Vec<u8>) -> Layout {
    assert!(
        encoding.resources.patches.is_empty(),
        "this resolve function doesn't support late bound resources"
    );
    let data = packed;
    data.clear();
    let mut layout = Layout {
        n_paths: encoding.n_paths,
        n_clips: encoding.n_clips,
        ..Layout::default()
    };
    let SceneBufferSizes {
        buffer_size,
        path_tag_padded,
    } = SceneBufferSizes::new(encoding);
    data.reserve(buffer_size);
    // Path tag stream
    layout.path_tag_base = size_to_words(data.len());
    data.extend_from_slice(bytemuck::cast_slice(&encoding.path_tags));
    for _ in 0..encoding.n_open_clips {
        data.extend_from_slice(bytemuck::bytes_of(&PathTag::PATH));
    }
    data.resize(path_tag_padded, 0);
    // Path data stream
    layout.path_data_base = size_to_words(data.len());
    data.extend_from_slice(bytemuck::cast_slice(&encoding.path_data));
    // Draw tag stream
    layout.draw_tag_base = size_to_words(data.len());
    // Bin data follows draw info
    layout.bin_data_start = encoding.draw_tags.iter().map(|tag| tag.info_size()).sum();
    data.extend_from_slice(bytemuck::cast_slice(&encoding.draw_tags));
    for _ in 0..encoding.n_open_clips {
        data.extend_from_slice(bytemuck::bytes_of(&DrawTag::END_CLIP));
    }
    // Draw data stream
    layout.draw_data_base = size_to_words(data.len());
    data.extend_from_slice(bytemuck::cast_slice(&encoding.draw_data));
    // Transform stream
    layout.transform_base = size_to_words(data.len());
    data.extend_from_slice(bytemuck::cast_slice(&encoding.transforms));
    // Style stream
    layout.style_base = size_to_words(data.len());
    data.extend_from_slice(bytemuck::cast_slice(&encoding.styles));
    layout.n_draw_objects = layout.n_paths;
    assert_eq!(buffer_size, data.len());
    layout
}

/// Resolver for late bound resources.
#[derive(Default)]
pub struct Resolver {
    image_cache: ImageCache,
    pending_images: Vec<PendingImage>,
    patches: Vec<ResolvedPatch>,
}

impl Resolver {
    /// Creates a new resource cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves late bound resources and packs an encoding. Returns the packed
    /// layout and computed ramp data.
    pub fn resolve<'a>(
        &'a mut self,
        encoding: &Encoding,
        packed: &mut Vec<u8>,
    ) -> (Layout, Images<'a>) {
        let resources = &encoding.resources;
        if resources.patches.is_empty() {
            let layout = resolve_solid_paths_only(encoding, packed);
            return (layout, Images::default());
        }
        self.resolve_patches(encoding);
        self.resolve_pending_images();
        let data = packed;
        data.clear();
        let mut layout = Layout {
            n_paths: encoding.n_paths,
            n_clips: encoding.n_clips,
            ..Layout::default()
        };
        let SceneBufferSizes {
            buffer_size,
            path_tag_padded,
        } = SceneBufferSizes::new(encoding);
        data.reserve(buffer_size);
        // Path tag stream
        layout.path_tag_base = size_to_words(data.len());
        {
            data.extend_from_slice(bytemuck::cast_slice(&encoding.path_tags));

            for _ in 0..encoding.n_open_clips {
                data.extend_from_slice(bytemuck::bytes_of(&PathTag::PATH));
            }
            data.resize(path_tag_padded, 0);
        }
        // Path data stream
        layout.path_data_base = size_to_words(data.len());
        {
            data.extend_from_slice(bytemuck::cast_slice(&encoding.path_data));
        }
        // Draw tag stream
        layout.draw_tag_base = size_to_words(data.len());
        // Bin data follows draw info
        layout.bin_data_start = encoding.draw_tags.iter().map(|tag| tag.info_size()).sum();
        {
            data.extend_from_slice(bytemuck::cast_slice(&encoding.draw_tags));
            for _ in 0..encoding.n_open_clips {
                data.extend_from_slice(bytemuck::bytes_of(&DrawTag::END_CLIP));
            }
        }
        // Draw data stream
        layout.draw_data_base = size_to_words(data.len());
        {
            let mut pos = 0;
            let stream = &encoding.draw_data;
            for patch in &self.patches {
                let index = patch.index;
                let draw_data_offset = patch.draw_data_offset;

                if pos < draw_data_offset {
                    data.extend_from_slice(bytemuck::cast_slice(
                        &encoding.draw_data[pos..draw_data_offset],
                    ));
                }
                if let Some((x, y)) = self.pending_images[index].xy {
                    let xy = (x << 16) | y;
                    data.extend_from_slice(bytemuck::bytes_of(&xy));
                    pos = draw_data_offset + 1;
                } else {
                    // If we get here, we failed to allocate a slot for this image in the atlas.
                    // In this case, let's zero out the dimensions so we don't attempt to render
                    // anything.
                    // TODO: a better strategy: texture array? downsample large images?
                    data.extend_from_slice(&[0_u8; 8]);
                    pos = draw_data_offset + 2;
                }
            }
            if pos < stream.len() {
                data.extend_from_slice(bytemuck::cast_slice(&stream[pos..]));
            }
        }
        // Transform stream
        layout.transform_base = size_to_words(data.len());
        {
            data.extend_from_slice(bytemuck::cast_slice(&encoding.transforms));
        }
        // Style stream
        layout.style_base = size_to_words(data.len());
        {
            data.extend_from_slice(bytemuck::cast_slice(&encoding.styles));
        }

        layout.n_draw_objects = layout.n_paths;
        assert_eq!(buffer_size, data.len());
        (layout, self.image_cache.images())
    }

    fn resolve_patches(&mut self, encoding: &Encoding) {
        self.image_cache.clear();
        self.pending_images.clear();
        self.patches.clear();

        let resources = &encoding.resources;
        for patch in &resources.patches {
            let image = patch.image.clone();
            let draw_data_offset = patch.draw_data_offset;

            let index = self.pending_images.len();
            self.pending_images.push(PendingImage { image, xy: None });
            self.patches.push(ResolvedPatch {
                index,
                draw_data_offset,
            });
        }
    }

    fn resolve_pending_images(&mut self) {
        self.image_cache.clear();
        'outer: loop {
            // Loop over the images, attempting to allocate them all into the atlas.
            for pending_image in &mut self.pending_images {
                if let Some(xy) = self.image_cache.get_or_insert(&pending_image.image) {
                    pending_image.xy = Some(xy);
                } else {
                    // We failed to allocate. Try to bump the atlas size.
                    if self.image_cache.bump_size() {
                        // We were able to increase the atlas size. Restart the outer loop.
                        continue 'outer;
                    } else {
                        // If the atlas is already maximum size, there's nothing we can do. Set
                        // the xy field to None so this image isn't rendered and then carry on--
                        // other images might still fit.
                        pending_image.xy = None;
                    }
                }
            }
            // If we made it here, we've either successfully allocated all images or we reached
            // the maximum atlas size.
            break;
        }
    }
}

/// Patch for a late bound resource.
#[derive(Clone)]
pub struct Patch {
    /// Offset to the atlas coordinates in the draw data stream.
    pub draw_data_offset: usize,
    /// Underlying image data.
    pub image: Image,
}

/// Image to be allocated in the atlas.
#[derive(Clone, Debug)]
struct PendingImage {
    image: Image,
    xy: Option<(u32, u32)>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPatch {
    /// Index of pending image element.
    pub index: usize,
    /// Offset to the atlas location in the draw data stream.
    pub draw_data_offset: usize,
}

struct SceneBufferSizes {
    /// Full size of the scene buffer in bytes.
    buffer_size: usize,
    /// Padded length of the path tag stream in bytes.
    path_tag_padded: usize,
}

impl SceneBufferSizes {
    /// Computes common scene buffer sizes for the given encoding and patch
    /// stream sizes.
    fn new(encoding: &Encoding) -> Self {
        let n_path_tags = encoding.path_tags.len() + encoding.n_open_clips as usize;
        let path_tag_padded = align_up(n_path_tags, 4 * crate::config::PATH_REDUCE_WG);
        let buffer_size = path_tag_padded
            + slice_size_in_bytes(&encoding.path_data, 0)
            + slice_size_in_bytes(&encoding.draw_tags, encoding.n_open_clips as usize)
            + slice_size_in_bytes(&encoding.draw_data, 0)
            + slice_size_in_bytes(&encoding.transforms, 0)
            + slice_size_in_bytes(&encoding.styles, 0);
        Self {
            buffer_size,
            path_tag_padded,
        }
    }
}

fn slice_size_in_bytes<T: Sized>(slice: &[T], extra: usize) -> usize {
    (slice.len() + extra) * size_of::<T>()
}

fn size_to_words(byte_size: usize) -> u32 {
    (byte_size / size_of::<u32>()) as u32
}

fn align_up(len: usize, alignment: u32) -> usize {
    len + (len.wrapping_neg() & (alignment as usize - 1))
}
