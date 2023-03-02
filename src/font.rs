use std::{path::Path, fmt::Error, collections::HashMap};

use std::{fs, vec};
use crate::glyphs::GlyphMetrics;
use crate::layout_run::LayoutRun;

use owned_ttf_parser::{OwnedFace, Face, AsFaceRef, kern::{self, Subtable}};
use lopdf::{self, StringFormat};


#[derive(Debug)]
pub struct FontMetrics {
    pub ascent: i64,
    pub descent: i64,
    pub scale: f32,
    pub ascender_scaled: f32,
    pub descender_scaled: f32,
    pub line_gap: f32,
}

#[derive(Debug)]
pub struct Font<'s> {
    pub name: &'s str,
    face: OwnedFace,
    raw_bytes: Vec<u8>,
    pub units_per_em: u16,
    pub glyph_metrics: HashMap<u16, GlyphMetrics>,
    glyph_ids: HashMap<u16, char>,
    pub metrics: FontMetrics
}


impl Default for FontMetrics {
    fn default() -> Self {
        FontMetrics { ascent: 0, descent: 0, scale: 0.0, ascender_scaled: 0.0, descender_scaled: 0.0, line_gap: 0.0 }
    }
}



impl<'s> Font<'s> {
    pub fn new_from_file(src: &str, name: &'s str) -> Self {
        //load file
        let p = Path::new(src);
        //TODO: handle error at new_from_file
        let raw = fs::read(p).unwrap();
        let face = OwnedFace::from_vec(raw.clone(), 0).unwrap();
        let units_per_em = face.as_face_ref().units_per_em();
        let mut font = Self {
                name,
                raw_bytes: raw,
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
            ascent: self.face().ascender() as i64, 
            descent: self.face().descender()as i64, 
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

    pub fn line_height(&self, font_size: f32) -> f32 {
        let lh = self.calc_line_height();
        lh * font_size
    }

    pub fn line_gap(&self, font_size: f32) -> f32 {
        (self.metrics.line_gap / 1000.0) * font_size
    }


    pub fn layout_run(&self, text: &str, font_size: f32) -> Result<LayoutRun, Error> {
        let mut glyph_run = match self.run(text) {
            Ok(r) => r,
            Err(_) => panic!()
        };

        for glyph in glyph_run.iter_mut() {
            glyph.apply_kerning();
            glyph.apply_scale(self.metrics.scale);
            glyph.width *= font_size/1000.0;
        }
        Ok(LayoutRun {
            font: self.name.to_string(),
            text: text.to_string(),
            glyph_run,
            font_size,
            line_height: self.line_height(font_size),
            line_gap: self.line_gap(font_size),
        })
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
                metrics.push(glyph.clone());
            } else {
                if let Some(kerning) = kern_table.clone() {
                    glyph.kern_right = kerning.glyphs_kerning(self.glyph_id(previous as u16), self.glyph_id(current)).unwrap_or(0) as f32;
                }
                metrics.push(glyph.clone())
            }
        }
        Ok(metrics)
    }
    // almost directly taken from printpdf - except that we memcopy, font data
    pub fn embed_lopdf(&self, doc: &mut lopdf::Document, index: usize) -> lopdf::Dictionary 
        {
        use lopdf::Object;
        use lopdf::Object::*;

        let face_name = format!("F{}", index);

        let face_metrics = self.font_metrics();

        let bytes_len = self.raw_bytes.len();
        let mut buf = vec![0u8; bytes_len];
        buf.copy_from_slice(&self.raw_bytes);

        let font_stream = lopdf::Stream::new(
            lopdf::Dictionary::from_iter(vec![
                ("Length1", Integer(bytes_len as i64)),
            ]),
            buf, 
        ).with_compression(false);

        let mut font_vec: Vec<(::std::string::String, Object)> = vec![
            ("Type".into(), Name("Font".into())),
            ("Subtype".into(), Name("Type0".into())),
            ("BaseFont".into(), Name(face_name.clone().into_bytes())),
            ("Encodign".into(), Name("Identity-H".into())),
        ];

        let mut font_descriptor_vec: Vec<(::std::string::String, Object)> = vec![
            ("Type".into(),        Name("FontDescriptor".into())),
            ("fontName".into(),    Name(face_name.clone().into_bytes())),
            ("Ascent".into(),      Integer(face_metrics.ascent)),
            ("Descent".into(),     Integer(face_metrics.descent)),
            ("CapHeight".into(),   Integer(face_metrics.ascent)),
            ("ItalicAngle".into(), Integer(0)),
            ("Flags".into(),       Integer(32)),
            ("StemV".into(),       Integer(80)),
        ];

        let mut max_height = 0.0;
        let mut total_width = 0.0;
        let mut widths = Vec::<(u32, u32)>::new();
        let mut cmap = std::collections::BTreeMap::<u32, (u32, u32, u32)>::new();
        cmap.insert(0, (0, 1000, 1000));

        for (glyph_id, c) in self.glyph_ids.clone() {
            if let Some(metrics) = self.glyph_metrics_for_char(c) {

                if metrics.height > max_height {
                    max_height = metrics.height;
                }
                total_width += metrics.width;
                cmap.insert(glyph_id as u32, (c as u32, metrics.width as u32, metrics.height as u32));
            }
        }

        let mut cur_first_bit = 0_u16;

        let mut all_cmap_blocks = Vec::new(); 

        {
            let mut current_cmap_block = Vec::new();
            for (glyph_id, unicode_width_tuple) in &cmap {
                if (*glyph_id >> 8) as u16 != cur_first_bit || current_cmap_block.len() >= 100 {
                    all_cmap_blocks.push(current_cmap_block.clone());
                        current_cmap_block = Vec::new();
                        cur_first_bit = (*glyph_id >> 8) as u16;
                }

                let (unicode, width, _) = *unicode_width_tuple;
                current_cmap_block.push((*glyph_id, unicode));
                widths.push((*glyph_id, width));
            };
            
            all_cmap_blocks.push(current_cmap_block);
        }

        let cid_to_unicode_map =  generate_cid_to_unicode_map(face_name.clone(), all_cmap_blocks);
        
        let cid_to_unicode_map_stream = lopdf::Stream::new(lopdf::Dictionary::new(), cid_to_unicode_map.as_bytes().to_vec());
        let cid_to_unicode_map_stream_id = doc.add_object(cid_to_unicode_map_stream);



        let mut widths_list = Vec::<Object>::new();
        let mut current_low_gid = 0;
        let mut current_high_gid = 0;
        let mut current_width_vec = Vec::<Object>::new();

        let percentage_font_scaling = face_metrics.scale as f64;

        for (gid, width) in widths {
            if gid == current_high_gid {
                current_width_vec.push(Integer((width as f64 * percentage_font_scaling) as i64));
                current_high_gid += 1;
            } else {
                widths_list.push(Integer(current_low_gid as i64));
                widths_list.push(Array(current_width_vec.drain(..).collect()));
                
                current_width_vec.push(Integer((width as f64 * percentage_font_scaling) as i64));
                current_low_gid = gid;
                current_high_gid = gid + 1;
            }
        }
        widths_list.push(Integer(current_low_gid as i64));
        widths_list.push(Array(current_width_vec.drain(..).collect()));
        
        let w = ("W", Array(widths_list));
        let dw = ("DW", Integer(1000));

        let mut desc_fonts = lopdf::Dictionary::from_iter(vec![
            ("Type", Name("Font".into())),
            ("Subtype", Name("CIDFontType2".into())),
            ("BaseFont", Name(face_name.clone().into())),
            ("CIDSystemInfo", Dictionary(lopdf::Dictionary::from_iter(vec![
                ("Registry", String("Adobe".into(), StringFormat::Literal)),
                ("Ordering", String("Identity".into(), StringFormat::Literal)),
                ("Supplement", Integer(0))

            ]))),
            w, 
            dw
        ]);
 
        let font_bbox = vec![Integer(0), Integer(max_height as i64), Integer(total_width as i64), Integer(max_height as i64)];
        font_descriptor_vec.push(("FontFile2".into(), Reference(doc.add_object(font_stream))));
        font_descriptor_vec.push(("FontBBox".into(), Array(font_bbox)));

        let font_descriptor_vec_id = doc.add_object(lopdf::Dictionary::from_iter(font_descriptor_vec));
        desc_fonts.set("FontDescriptor", Reference(font_descriptor_vec_id));

        font_vec.push(("DescendantFonts".into(), Array(vec![Dictionary(desc_fonts)])));
        font_vec.push(("ToUnicode".into(), Reference(cid_to_unicode_map_stream_id)));

        lopdf::Dictionary::from_iter(font_vec)



    }





}

type GlyphId = u32;
type UnicodeCodePoint = u32;
type CmapBlock = Vec<(GlyphId, UnicodeCodePoint)>;

fn generate_cid_to_unicode_map(face_name: String, all_cmap_blocks: Vec<CmapBlock>) -> String {
    let mut cid_to_unicode_map = format!(include_str!("../assets/gid_to_unicode_beg.txt"), face_name);
    
    for cmap_block in all_cmap_blocks.into_iter().filter(|block| !block.is_empty() || block.len() < 100) {
        cid_to_unicode_map.push_str(format!("{} beginbfchar\r\n", cmap_block.len()).as_str());
        for (glyph_id, unicode) in cmap_block {
            cid_to_unicode_map.push_str(format!("<{:04x}> <{:04x}>\n", glyph_id, unicode).as_str());
        }

        cid_to_unicode_map.push_str("endbfchar\r\n");
    }
    cid_to_unicode_map.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend");
    cid_to_unicode_map
}












#[cfg(test)]
mod test {
    use lopdf::Document;

    use super::*;

   fn cmp_f32(this: f32, that: f32) -> bool {
        (this - that).abs() < 0.1f32  
    }

    const NOTO: &str = "assets/NotoSansRegularNerdFontComplete.ttf";
    const CAL: &str = "assets/Calibri Regular.ttf";
    #[test]
    fn init_font() {
        let f = Font::new_from_file(NOTO, "noto");
        assert_eq!(f.name, "noto");
        assert_eq!(f.units_per_em, 1000);
        assert_eq!(f.font_metrics().ascent, 1069);
        assert_eq!(f.font_metrics().descent, -293);

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
        assert!(cmp_f32(f.line_gap(16.0), 3.53125f32));
    }


    #[test]
    fn get_kern_table() {
       let f = Font::new_from_file(CAL, "Calibri");
       let kerning = f.retrieve_kern_table();
       assert!(kerning.is_some());
    }

    #[test]
    fn get_layout_run() {
        let f = Font::new_from_file(CAL, "Calibri");
        let text = "AB";
        let expected = LayoutRun { 
            text: "AB".to_string(), 
            font: "Calibri".to_string(), 
            font_size: 12.0, 
            glyph_run: vec![
                GlyphMetrics { 
                    id: 4, 
                    character: 'A', 
                    width: 6.9433594, 
                    height: 1818.0, 
                    kern_right: 0.0 
                }, 
                GlyphMetrics { 
                    id: 17, 
                    character: 'B',
                    width: 6.5273438, 
                    height: 1806.0, 
                    kern_right: 0.0 
                }
            ], 
            line_height: 14.6484375, 
            line_gap: 2.6484375 
        };
        assert_eq!(f.layout_run(text, 12.0), Ok(expected));
    }

    #[test]
    fn test_embedding() {
        let f = Font::new_from_file(CAL, "Calibri");
        let mut doc = Document::with_version("1.7");
        f.embed_lopdf(&mut doc, 0);
    }
}









