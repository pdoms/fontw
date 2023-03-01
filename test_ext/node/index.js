const fontkit = require("fontkit")
const fs = require("fs")
const util = require("util")


const usage = () => {
    console.log(`
node index.js -f <src> <font> <fontsize> <out>
    
    <src> ......... src string
    <font> ........ path to font file [.ttf or .otf]
    <fontsize> .... fontsize in pt [defaults to 12pt]
    <out> ......... file to write to [if none stdout/console.log]
    -f ........ ... interprets <src> as path to file if present 
`)
}

let args = process.argv
let src = null;
let font_path = null;
let out = "stdout";
let is_file = false;
let font_size = 12;

if (args.length < 5) {
    usage()
    console.log("[ERROR] not enough arguments provided")
    process.exit(1)
} else {
    if (args[2] == "-f") {
        src = args[3] 
        is_file = true
        font_path = args[4]
        if (args.length === 6) {
            font_size = parseFloat(args[5])
        }
        if (args.length === 7) {
            font_size = parseFloat(args[5])
            out = args[6]
        }
    } else {
        src = args[2]
        font_path = args[3]
        if (args.length === 5) {
            font_size = parseFloat(args[4])
        }
        if (args.length === 6) {
            font_size = parseFloat(args[4])
            out = args[5]
        }
    }
    if (src === null) {
        usage()
        console.log("[ERROR] unparsable arguments - could not identify src")
    }
    if (font_path === null) {
        usage()
        console.log("[ERROR] unparsable arguments - could not identify font")
    }
    console.log(src, font_path, font_size, out)
}

function load_file(path) {
    return fs.readFileSync(path).toString()
}

function code_points_to_char(codepoints) {
    let vals = "0x"
    for (let i = 0; i < codepoints.length; i++) {
        vals += codepoints[i].toString(16)
    }
    return String.fromCodePoint(vals)
}

function representation_to_metrics(rep) {
    let metrics = []
    rep.forEach((item) => {
        metrics.push({value: item.text, width: item.widths.reduce((acc, curr) => acc + curr)})
    })
    return metrics
}


function run() {
    let str_src; 
    let font;
    if (is_file) {
        str_src = load_file(src)
    } else {
        str_src = src
    }
    if (font_path.endsWith(".ttf") || font_path.endsWith(".otf")) {
         font = fontkit.openSync(font_path);
    } else {
        console.log("[ERROR] wrong font file format")
        process.exit(1)
    }
    if (!str_src || !font) {
        console.log("[ERROR] something went wrong with retrieving the font or soruce")
        process.exit(1)
    }

    let name = font.postscriptName;
    let scale = 1000 / font.unitsPerEm;
    let ascender = font.ascent * scale
    let descender = font.descent * scale;
    let xHeight = font.xHeight * scale;
    let capHeight = font.capHeight * scale;
    let lineGap = font.lineGap * scale;
    let bbox = font.bbox;
    
    let metrics = {name}
    //prepare run
    let all_run_raw = font.layout(str_src, {
        rvrn: false,
        ltra: false,
        ltrm: false,
        frac: false,
        numr: false,
        dnom: false,
        ccmp: false,
        locl: false,
        rlig: false,
        mark: false,
        mkmk: false,
        calt: false,
        clig: false,
        liga: false,
        rclt: false,
        curs: false,
        kern: false

    })
    for (let i = 0; i < all_run_raw.positions.length; i++) {
        const all_position = all_run_raw.positions[i];
        for (let key in all_position) {
            all_position[key] *= scale;
        }
        all_position.advanceWidth = all_run_raw.glyphs[i].advanceWidth * scale;
    }
    //console.log(util.inspect(all_run_raw, false, 3))

    let _scale = font_size / 1000;
    //take whole string width regardles of lines
    metrics["text"] = str_src
    metrics["total_width"] = all_run_raw.advanceWidth * _scale
    metrics["line_gap"] = lineGap
    
    //as lines and words
    let nl = font.glyphForCodePoint('\n'.charCodeAt(0))
    let ws = font.glyphForCodePoint(' '.charCodeAt(0))
    let lines = []
    let line = {glyphs: [], widths: [], text: ""}
    let words = []
    let word = {glyphs: [], widths: [], text: ""}
    for (let i = 0; i < all_run_raw.glyphs.length; i++) {
        let glyph = all_run_raw.glyphs[i]
        let position = all_run_raw.positions[i]
        if (nl.id == glyph.id) {
            lines.push(line)
            line = {glyphs: [], widths: [], text: ""} 
        } else {
            line.glyphs.push(glyph.id)
            line.widths.push(position.advanceWidth * _scale)
            line.text += code_points_to_char(glyph.codePoints)
        }
        if (ws.id == glyph.id || nl.id === glyph.id) {
            words.push(word)
            word = {glyphs: [], widths: [], text: ""} 
        } else {
            word.glyphs.push(glyph.id)
            word.widths.push(position.advanceWidth * _scale)
            word.text += code_points_to_char(glyph.codePoints)
        }
    }
    
    lines.push(line);
    words.push(word)

    
    metrics["lines"] = representation_to_metrics(lines)
    metrics["words"] = representation_to_metrics(words)
   
    if (out === "stdout") {
        process.stdout.write(JSON.stringify(metrics))
    } else {
        fs.writeFileSync(out, JSON.stringify(metrics))
    }
    console.log("[INFO] JOB DONE")
    process.exit(0)
    
}

run()












