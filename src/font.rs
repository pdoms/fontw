use std::path::Path;
use std::collections::HashMap;
use std::fs;

use owned_ttf_parser::{OwnedFace, Face, AsFaceRef, kern};

#[derive(Debug)]
pub struct GlyphMetrics {
   id: u16,
   character: char,
   width: u32,
   height: u32
}

#[derive(Debug)]
pub struct FontMetrics {
    pub ascent: i16,
    pub descent: i16,
    pub units_per_em: u16,
    pub scale: f64,
    pub ascender_scaled: f64,
    pub descender_scaled: f64,
    // pub x_height: f64,
    // pub capHeight: f64,
    // pub line_gap: f64
}

#[derive(Debug)]
pub struct Font<'s> {
    name: &'s str,
    face: OwnedFace,
    units_per_em: u16,
    glyph_metrics: HashMap<u16, GlyphMetrics>,
    glyph_ids: HashMap<u16, char>,
}




impl<'s> Font<'s> {
    pub fn new_from_file(src: &str, name: &'s str) -> Self {
        //load file
        let p = Path::new(src);
        //TODO: handle error at new_from_file
        let raw = fs::read(p).unwrap();
        let face = OwnedFace::from_vec(raw, 0).unwrap();
        let units_per_em = face.as_face_ref().units_per_em();
        let mut font = Self {
                name,
                face,
                units_per_em,
                glyph_metrics: HashMap::new(),
                glyph_ids: HashMap::new(),

            };
        font.get_glyphs_data();
        font

        }
    fn face(&self) -> &Face<'_> {
        self.face.as_face_ref()
    }
    pub fn font_metrics(&self) -> FontMetrics {
        let scale =  1000.0 / (self.face().units_per_em() as f64);
        FontMetrics { 
            ascent: self.face().ascender(), 
            descent: self.face().descender(), 
            units_per_em: self.face().units_per_em(),
            scale,
            ascender_scaled: (self.face().ascender() as f64) * scale, 
            descender_scaled: (self.face().descender() as f64) * scale, 
        }
            
    }

    pub fn glyph_id_for_char(&self, c: char) -> Option<u16> {
        self.face() 
            .glyph_index(c)
            .map(|id| id.0)
    }

    pub fn glyph_metrics_for_char(&self, c: char) -> Option<&GlyphMetrics> {
        if let Some(id) = self.glyph_id_for_char(c) {
            if let Some(met) = self.glyph_metrics.get(&id) {
                Some(met)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_glyphs_data(&mut self) {
        let subtables = self.face()
            .tables()
            .cmap
            .unwrap()
            .subtables
            .into_iter()
            .filter(|s| s.is_unicode());
        let capacity = self.face.as_face_ref().number_of_glyphs().into();
        let mut ids = HashMap::with_capacity(capacity);
        let mut mertrics = HashMap::with_capacity(capacity);
        for sub in subtables {
            sub.codepoints(|c| {
                use std::convert::TryFrom as _;

                if let Ok(ch) = char::try_from(c) {
                    if let Some(idx) = sub.glyph_index(c).filter(|idx| idx.0 > 0) {
                        ids.entry(idx.0).or_insert(ch);
                        if let Some(met) = self.get_glyph_metrics(idx.0, ch){
                            mertrics.entry(idx.0).or_insert(met);
                        };
                    }
                }
            })
        }
        self.glyph_ids = ids;
        self.glyph_metrics = mertrics;
    }

    fn get_glyph_metrics(&self, glyph_id_in: u16, ch: char) -> Option<GlyphMetrics> {
        let glyph_id = owned_ttf_parser::GlyphId(glyph_id_in);
        if let Some(width) = self.face().glyph_hor_advance(glyph_id) {
            let width = width as u32;
            let height = self.face().glyph_bounding_box(glyph_id).map(|bbox| {
                bbox.y_max - bbox.y_min - self.face().descender()
            }).unwrap_or(1000) as u32;
            Some(GlyphMetrics::new(glyph_id_in, width, height, ch))
        } else {
            None
        }
    }

    //fn get_kerning_for_char(&self, left: char, right: char) {}
    
    fn get_kerning_for_glyph_ids(&self, left: u16, right: u16)  {
        //what about GPOS
        let left = owned_ttf_parser::GlyphId(left);
        let right = owned_ttf_parser::GlyphId(right);
        //TODO ERRORS and OPTIONS
        let kern_tables = self.face().tables().kern.unwrap().subtables.into_iter();       
        for (i, k)in kern_tables.enumerate() {
            println!("{:?}", i);
        }
    }


    fn get_text_width(&self, text: &str, font_size: f64) -> f64 {
        let run =        

        


        0.0
           
    }

    fn get_run(&self, text: &str) ->  {
        text.chars().map(|c| {self.glyph_metrics_for_char(c)});
    }
}



impl GlyphMetrics {
    pub fn new(id: u16, width: u32, height: u32, character: char) -> Self {
        Self {
            id, width, height, character
        }
    }

    pub fn id(&self) -> u16 {
        self.id
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn character(&self) -> char {
        self.character
    }
}



#[cfg(test)]
mod test {

    use super::*;
    const SRC: &str = "/usr/share/fonts/nerd-fonts-complete/TTF/Noto Sans Regular Nerd Font Complete.ttf";
    const NAME: &str = "noto";

    #[test]
    fn init_font() {
        let f = Font::new_from_file(SRC, NAME);
        assert_eq!(f.name, "noto");
        assert_eq!(f.units_per_em, 1000);
        assert_eq!(f.font_metrics().ascent, 1069);
        assert_eq!(f.font_metrics().descent, -293);

    }

    #[test]
    fn get_glypth_id() {
        let f = Font::new_from_file(SRC, NAME);
        assert_eq!(f.glyph_id_for_char('a'), Some(70));

    }

    #[test]
    fn get_glyph_metrics() {

        // Values can very by 1, because owned_ttf_parser/ttf_parser don't round 
        // but rather offset [because of no-std]
        let f = Font::new_from_file(SRC, NAME);
        let glyph_metrics = f.glyph_metrics_for_char('a');
        let height = 545 - (-10 as i32) - (-293 as i32);
        let advanced_width: u32 = 561;
        let id = 70;
        assert_eq!(glyph_metrics.unwrap().id(), id);
        assert_eq!(glyph_metrics.unwrap().height(), height as u32);
        assert_eq!(glyph_metrics.unwrap().width(), advanced_width);
        assert_eq!(glyph_metrics.unwrap().character, 'a');
    }

    #[test]
    fn get_kerning() {
        let f = Font::new_from_file("/home/pd/projects/fontw/Calibri Regular.ttf", "Calibri");
        f.get_kerning_for_glyph_ids(70, 71);
    }
}
