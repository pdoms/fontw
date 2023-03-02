use crate::glyphs::GlyphMetrics;


#[derive(Debug, Clone, PartialEq)]
pub struct LayoutRun {
    pub text: String,
    pub font: String,
    pub font_size: f32,
    pub glyph_run: Vec<GlyphMetrics>,
    pub line_height: f32,
    pub line_gap: f32,
}

pub struct WordRun {}
pub struct LineRun {}


impl LayoutRun {
    pub fn run_width(&self) -> f32 {
        let mut acc = 0.0;
        for glyph in self.glyph_run.iter() {
            acc += glyph.width;
        } 
        acc
    }


}




#[cfg(test)]
mod test {

    use crate::font::Font;

    fn cmp_f32(this: f32, that: f32) -> bool {
        (this - that).abs() < 0.1f32  
    }

    const CAL: &str = "assets/Calibri Regular.ttf";
    #[test]
    fn test_run_width() {
        let f = Font::new_from_file(CAL, "Calibri");
        let text = "Hello World";
        let run = f.layout_run(text, 12.0).unwrap();
        let expected_width = 58.242188f32;
        assert!(cmp_f32(run.run_width(), expected_width));
    }
}

