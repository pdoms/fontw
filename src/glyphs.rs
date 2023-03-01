#[derive(Debug, Clone, PartialEq)]
pub struct GlyphMetrics {
   pub id: u16,
   pub character: char,
   pub width: f32,
   pub height: f32,
   pub kern_right: f32,
}
impl GlyphMetrics {
    pub fn new(id: u16, width: f32, height: f32, character: char) -> Self {
        Self {
            id, width, height, character, kern_right: 0.0
        }
    }

    pub fn apply_kerning(&mut self){
        self.width += self.kern_right
    }

    pub fn apply_scale(&mut self, scale: f32) {
        self.width *= scale;
    }
}
