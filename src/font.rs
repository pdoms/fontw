use std::ops::Deref;
use std::{path::Path, fmt::Error};
use std::collections::HashMap;
use std::fs;

use owned_ttf_parser::{OwnedFace, Face, AsFaceRef, kern::{self, Subtable}};

#[derive(Debug, Clone, PartialEq)]
pub struct GlyphMetrics {
   pub id: u16,
   pub character: char,
   pub width: f32,
   pub height: f32,
   pub kern_right: f32,
}

#[derive(Debug)]
pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub scale: f32,
    pub ascender_scaled: f32,
    pub descender_scaled: f32,
    pub line_gap: f32,
}

#[derive(Debug)]
pub struct Font<'s> {
    name: &'s str,
    face: OwnedFace,
    units_per_em: u16,
    glyph_metrics: HashMap<u16, GlyphMetrics>,
    glyph_ids: HashMap<u16, char>,
    metrics: FontMetrics
}

pub struct LayoutRun<'t> {
    text: &'t str,
    font: &'t str,
    font_size: f32,
    glyph_run: Vec<GlyphMetrics>,
    line_height: f32,
    line_gap: f32,
}

impl Default for FontMetrics {
    fn default() -> Self {
        FontMetrics { ascent: 0.0, descent: 0.0, scale: 0.0, ascender_scaled: 0.0, descender_scaled: 0.0, line_gap: 0.0 }
    }
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
                metrics: FontMetrics::default(),
            };
        font.calc_glyphs_data();
        font.metrics = font.font_metrics();
        font

        }
    fn face(&self) -> &Face<'_> {
        self.face.as_face_ref()
    }
    pub fn font_metrics(&self) -> FontMetrics {
        let scale =  1000.0 / (self.face().units_per_em() as f32);
        FontMetrics { 
            ascent: self.face().ascender() as f32, 
            descent: self.face().descender()as f32, 
            scale,
            ascender_scaled: (self.face().ascender() as f32) * scale, 
            descender_scaled: (self.face().descender() as f32) * scale, 
            line_gap: (self.face().line_gap()  as f32) * scale
        }
            
    }

    pub fn glyph_id_for_char(&self, c: char) -> Option<u16> {
        self.face() 
            .glyph_index(c)
            .map(|id| id.0)
    }

    fn id_by_char(&self, c: char) -> Option<owned_ttf_parser::GlyphId> {
        if let Some(id) = self.glyph_id_for_char(c) {
            Some(owned_ttf_parser::GlyphId(id))
        } else {
            None
        }
    } 
    fn glyph_id(&self, id: u16) -> owned_ttf_parser::GlyphId {
        owned_ttf_parser::GlyphId(id)
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

    fn calc_glyphs_data(&mut self) {
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
                        if let Some(met) = self.calc_glyph_metrics(idx.0, ch){
                            mertrics.entry(idx.0).or_insert(met);
                        };
                    }
                }
            })
        }
        self.glyph_ids = ids;
        self.glyph_metrics = mertrics;
    }

    fn calc_glyph_metrics(&self, glyph_id_in: u16, ch: char) -> Option<GlyphMetrics> {
        let glyph_id = owned_ttf_parser::GlyphId(glyph_id_in);
        if let Some(width) = self.face().glyph_hor_advance(glyph_id) {
            let width = width as f32;
            let height = self.face().glyph_bounding_box(glyph_id).map(|bbox| {
                bbox.y_max - bbox.y_min - self.face().descender()
            }).unwrap_or(1000) as f32;
            Some(GlyphMetrics::new(glyph_id_in, width, height, ch))
        } else {
            None
        }
    }
    

    fn calc_line_height(&self) -> f32 {
        let gap = self.metrics.line_gap;
        let asc = self.metrics.ascender_scaled;
        let dsc = self.metrics.descender_scaled;
        (asc + gap - dsc) / 1000.0
    }

    fn line_height(&self, font_size: f32) -> f32 {
        let lh = self.calc_line_height();
        lh * font_size
    }


    fn layout_run(&self, text: &str, font_size: f32) -> Result<f32, Error> {
        let mut glyph_run = match self.run(text) {
            Ok(r) => r,
            Err(_) => panic!()
        };

        





        let layout = LayoutRun {
            font: self.name,
            text,
            glyph_run,
            font_size,
            line_height: 0.0,
            line_gap: 0.0
        };





        Ok(0.0)
    }

    fn retrieve_kern_table(&self) -> Option<Subtable> {
        if let Some(kern) = self.face().tables().kern {
            kern.subtables.into_iter().next()
        } else {
            None
        }
    }

    fn run(&self, text: &str) -> Result<Vec<GlyphMetrics>, Error> {
        let mut previous = -1;
        let kern_table = self.retrieve_kern_table();
        let mut metrics = Vec::new();
        for c in text.chars() { 
            let current = self.glyph_id_for_char(c).unwrap();
            let mut glyph = match self.glyph_metrics_for_char(c) {
                Some(g) => g.clone(),
                None => panic!()
            };
            if previous == -1 {
                previous = current as i32;
                metrics.push(glyph);
            } else {
                if let Some(kerning) = kern_table.clone() {
                    glyph.kern_right = kerning.glyphs_kerning(self.glyph_id(previous as u16), self.glyph_id(current)).unwrap_or(0) as f32;
                }
                metrics.push(glyph)
            }
        }
        Ok(metrics)
    }
}



impl GlyphMetrics {
    pub fn new(id: u16, width: f32, height: f32, character: char) -> Self {
        Self {
            id, width, height, character, kern_right: 0.0
        }
    }
}




#[cfg(test)]
mod test {

    use super::*;

    fn cmp_f32(this: f32, that: f32) -> bool {
        (this - that).abs() < 0.1f32  
    }

    const NOTO: &str = "/usr/share/fonts/nerd-fonts-complete/TTF/Noto Sans Regular Nerd Font Complete.ttf";
    const CAL: &str = "Calibri Regular.ttf";
    #[test]
    fn init_font() {
        let f = Font::new_from_file(NOTO, "noto");
        assert_eq!(f.name, "noto");
        assert_eq!(f.units_per_em, 1000);
        assert_eq!(f.font_metrics().ascent, 1069.0);
        assert_eq!(f.font_metrics().descent, -293.0);

    }

    #[test]
    fn get_glypth_id() {
        let f = Font::new_from_file(NOTO, "noto");
        assert_eq!(f.glyph_id_for_char('a'), Some(70));

    }

    #[test]
    fn get_glyph_metrics() {
        // Values can very by 1, because owned_ttf_parser/ttf_parser don't round 
        // but rather offset [because of no-std]
        let f = Font::new_from_file(NOTO, "noto");
        let glyph_metrics = f.glyph_metrics_for_char('a');
        let height = 545 - (-10 as i32) - (-293 as i32);
        let advanced_width: f32 = 561.0;
        let id = 70;
        assert_eq!(glyph_metrics.unwrap().id, id);
        assert_eq!(glyph_metrics.unwrap().height, height as f32);
        assert_eq!(glyph_metrics.unwrap().width, advanced_width);
        assert_eq!(glyph_metrics.unwrap().character, 'a');
    }



    #[test]
    fn get_glyph_run() {
        let f = Font::new_from_file(CAL, "Calibri");
        let text = "AB";
        let expected = vec![GlyphMetrics { id: 4, character: 'A', width: 1185.0, height: 1818.0, kern_right: 0.0 }, GlyphMetrics { id: 17, character: 'B', width: 1114.0, height: 1806.0, kern_right: 0.0 }];
        let run = f.run(text).unwrap();
        assert_eq!(run, expected);
    }

    #[test]
    fn test_line_height() {
        let f = Font::new_from_file(CAL, "Calibri");
        assert!(cmp_f32(f.calc_line_height(), 1.220703f32));
        assert!(cmp_f32(f.line_height(16.0), 19.53125f32));
    }








    #[test]
    fn get_kern_table() {
        let f = Font::new_from_file(CAL, "Calibri");
       let kerning = f.retrieve_kern_table();
       assert!(kerning.is_some());
    }
}









