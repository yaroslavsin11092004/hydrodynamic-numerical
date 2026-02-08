use freetype as ft;
use freetype::freetype_sys as fts;
use glam;

#[derive(Default, Clone)]
pub struct Glyph {
    pub uv_min: glam::Vec2,
    pub uv_max: glam::Vec2,
    pub width: u32,
    pub height: u32,
    pub bearing: glam::Vec2,
    pub advance: i64
}

#[derive(Default, Clone)]
pub struct AtlasPosition {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32 
}
pub struct GlyphData {
    pub codepoint: u32,
    pub width: u32,
    pub height: u32,
    pub bitmap: Vec<u8>,
    pub pitch: i32,
    pub pixel_mode: fts::FT_Pixel_Mode,
    pub bearing: (f32, f32),
    pub advance: i64,
    pub left: i32,
    pub top: i32,
    pub row: u32,
    pub col: u32
}
impl GlyphData {
    pub fn new(codepoint: u32, face: &ft::Face) -> Result<Self, ft::Error> {
        face.load_char(codepoint as usize, ft::face::LoadFlag::RENDER).expect("Failed to load char");
        let glyph = face.glyph();
        let bitmap = glyph.bitmap();
        let raw_bitmap = unsafe { &*(bitmap.raw() as *const fts::FT_Bitmap) };
        let width = raw_bitmap.width as u32;
        let height = raw_bitmap.rows as u32;
        let pitch = raw_bitmap.pitch;
        let pixel_mode =  raw_bitmap.pixel_mode as u32;
        let mut bitmap_data = Vec::new();
        if width > 0 && height > 0 && !raw_bitmap.buffer.is_null() {
            let buffer_size = (height as usize) * pitch.unsigned_abs() as usize;
            bitmap_data.resize(buffer_size, 0);
            unsafe {
                std::ptr::copy_nonoverlapping(raw_bitmap.buffer, bitmap_data.as_mut_ptr(), buffer_size)
            };
        };
        Ok(Self {
            codepoint, 
            width, 
            height,
            bitmap: bitmap_data,
            pitch: pitch,
            pixel_mode,
            bearing: (
                glyph.bitmap_left() as f32,
                (glyph.bitmap_top() as f32) - height as f32 
            ),
            advance: glyph.advance().x as i64,
            left: glyph.bitmap_left(),
            top: glyph.bitmap_top(),
            row: 0,
            col: 0
        })
    }
    pub fn get_pixel(&self, x: u32, y: u32) -> u8 {
        if x >= self.width || y >= self.height || self.bitmap.is_empty() {
            return 0;
        };
        let row_offset = (y as usize) * self.pitch.unsigned_abs() as usize;
        let col_offset = x as usize;
        match self.pixel_mode {
            fts::FT_PIXEL_MODE_MONO => {
                let byte_offset = row_offset + (col_offset / 8);
                let bit = 7 - (col_offset % 8);
                let byte = self.bitmap[byte_offset];
                if (byte & (1 << bit)) != 0 { 255 } else { 0 }
            },
            fts::FT_PIXEL_MODE_GRAY => {
                self.bitmap[row_offset + col_offset]
            }
            _ => 0 
        }
    }
}

pub fn strip_pack(glyphs: &[GlyphData], atlas_width: u32, atlas_height: u32) -> Vec<AtlasPosition> {
    let mut positions = vec![AtlasPosition::default(); glyphs.len()];
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut row_height: u32 = 0;
    for i in 0..glyphs.len() {
        let glyph = &glyphs[i];
        if x + glyph.width > atlas_width {
            x = 0;
            y += row_height;
            row_height = 0;
            if y + glyph.height > atlas_height {
                panic!("Atlas overflow!")
            };
        };
        if y + glyph.height > atlas_height {
            panic!("Atlas height exceeded!");
        };
        positions[i] = AtlasPosition {
            x: x,
            y: y,
            width: glyph.width,
            height: glyph.height 
        };
        x += glyph.width;
        if glyph.height > row_height {
            row_height = glyph.height;
        };
    };
    positions
}
