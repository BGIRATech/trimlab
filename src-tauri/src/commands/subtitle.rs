// ============================================================
// FICHIER : src-tauri/src/commands/subtitle.rs
// ROLE    : Export SRT, ASS (karaoké), burn-in FFmpeg sous-titres
// ============================================================

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

// ── Types partagés ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
pub struct SubtitleWord {
    pub word:  String,
    pub start: f64,
    pub end:   f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SubtitleBlock {
    pub start: f64,
    pub end:   f64,
    pub text:  String,
    #[serde(default)]
    pub words: Vec<SubtitleWord>,
    #[serde(default)]
    pub edited: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SubtitleStyle {
    #[serde(rename = "fontFamily")]       pub font_family:       String,
    #[serde(rename = "fontSize")]         pub font_size:         u32,
    pub color:                            String,
    #[serde(rename = "outlineColor")]     pub outline_color:     String,
    #[serde(rename = "outlineWidth")]     pub outline_width:     f32,
    #[serde(rename = "bgColor")]          pub bg_color:          String,
    #[serde(rename = "bgOpacity")]        pub bg_opacity:        f32,
    pub position:                         String,
    pub bold:                             bool,
    pub italic:                           bool,
    #[serde(rename = "karaokeHighlight")] pub karaoke_highlight: String,
    #[serde(rename = "karaokeEnabled")]   pub karaoke_enabled:   bool,
    pub animation:                        String,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// "#RRGGBB" + alpha byte → "&HAABBGGRR" (format ASS complet)
fn hex_to_ass_alpha(hex: &str, alpha: u8) -> String {
    let h = hex.trim_start_matches('#');
    if h.len() >= 6 {
        let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(255);
        format!("&H{:02X}{:02X}{:02X}{:02X}", alpha, b, g, r)
    } else {
        format!("&H{:02X}FFFFFF", alpha)
    }
}

/// Couleur opaque (alpha=0x00)
fn hex_to_ass(hex: &str) -> String { hex_to_ass_alpha(hex, 0x00) }

/// opacity 0.0-1.0 → alpha ASS (0x00=opaque, 0xFF=transparent)
fn opacity_to_alpha(opacity: f32) -> u8 {
    ((1.0 - opacity.clamp(0.0, 1.0)) * 255.0).round() as u8
}

fn secs_to_srt(t: f64) -> String {
    let ms = (t * 1000.0).round() as u64;
    format!("{:02}:{:02}:{:02},{:03}", ms/3600000, (ms/60000)%60, (ms/1000)%60, ms%1000)
}

fn secs_to_ass(t: f64) -> String {
    let cs = (t * 100.0).round() as u64;
    format!("{}:{:02}:{:02}.{:02}", cs/360000, (cs/6000)%60, (cs/100)%60, cs%100)
}

fn output_path(project_id: &str, ext: &str) -> Result<PathBuf, String> {
    let base = dirs::document_dir().or_else(|| dirs::home_dir()).unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("TrimLab").join("exports");
    fs::create_dir_all(&dir).map_err(|e| format!("Impossible de créer le dossier exports: {}", e))?;
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    Ok(dir.join(format!("{}_{}.{}", project_id, ts, ext)))
}

// ── Générateur ASS partagé ────────────────────────────────────────────────────
// [FIX] BorderStyle=3 pour fond plein, calcul alpha correct via hex_to_ass_alpha

// Probe résolution réelle via ffprobe, fallback 1920x1080
fn probe_video_dimensions(video_path: &str) -> (u32, u32) {
    if let Ok(o) = std::process::Command::new("ffprobe")
        .args(["-v","error","-select_streams","v:0",
               "-show_entries","stream=width,height","-of","csv=p=0", video_path])
        .output()
    {
        let s = String::from_utf8_lossy(&o.stdout);
        let p: Vec<&str> = s.trim().split(',').collect();
        if p.len() == 2 {
            if let (Ok(w), Ok(h)) = (p[0].parse::<u32>(), p[1].parse::<u32>()) {
                if w > 0 && h > 0 { return (w, h); }
            }
        }
    }
    (1920, 1080)
}

// Fusionne les segments keep qui se chevauchent (gap < 50ms)
fn merge_keeps(segs: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if segs.is_empty() { return vec![]; }
    let mut v: Vec<(f64,f64)> = segs.to_vec();
    v.sort_by(|a,b| a.0.partial_cmp(&b.0).unwrap());
    let mut out: Vec<(f64,f64)> = Vec::new();
    let (mut cs, mut ce) = v[0];
    for &(s,e) in &v[1..] {
        if s <= ce + 0.05 { ce = ce.max(e); }
        else { if ce-cs > 0.01 { out.push((cs,ce)); } cs=s; ce=e; }
    }
    if ce-cs > 0.01 { out.push((cs,ce)); }
    out
}

// Remappe un timestamp source → timestamp dans la vidéo trimée
// Si t tombe dans un silence coupé, le snape au début du segment suivant
fn remap_time(t: f64, keeps: &[(f64, f64)]) -> f64 {
    let mut offset = 0.0_f64;
    for &(s, e) in keeps {
        if t <= e {
            return offset + (t.max(s) - s);
        }
        offset += e - s;
    }
    offset // snap à la fin
}

fn build_ass(blocks: &[SubtitleBlock], s: &SubtitleStyle, video_w: u32, video_h: u32) -> String {
    let alignment: u8 = match s.position.as_str() {
        "top"    => 8,
        "middle" => 5,
        _        => 2,
    };
    let margin_v: u32 = match s.position.as_str() {
        "middle" => 0,
        _        => 60,
    };

    let primary_color   = hex_to_ass(&s.color);
    let secondary_color = hex_to_ass(&s.karaoke_highlight);
    let outline_color   = hex_to_ass(&s.outline_color);
    let bg_alpha        = opacity_to_alpha(s.bg_opacity);
    let back_color      = hex_to_ass_alpha(&s.bg_color, bg_alpha);
    let border_style    = if s.bg_opacity > 0.01 { 3 } else { 1 };
    let outline_val: f32 = if border_style == 3 { (s.outline_width + 3.0).max(4.0) }
                           else { s.outline_width };
    let bold_flag   = if s.bold   { "-1" } else { "0" };
    let italic_flag = if s.italic { "-1" } else { "0" };

    let mut out = format!(
        "[Script Info]\n\
         ScriptType: v4.00+\n\
         PlayResX: {pw}\n\
         PlayResY: {ph}\n\
         ScaledBorderAndShadow: yes\n\n\
         [V4+ Styles]\n\
         Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, \
                 Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, \
                 Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
         Style: Default,{font},{size},{pri},{sec},{out},{back},\
{bold},{ital},0,0,100,100,0,0,{bs},{ov:.1},0,{align},20,20,{mv},1\n\n\
         [Events]\n\
         Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        pw    = video_w,
        ph    = video_h,
        font  = s.font_family,
        size  = s.font_size,
        pri   = primary_color,
        sec   = secondary_color,
        out   = outline_color,
        back  = back_color,
        bold  = bold_flag,
        ital  = italic_flag,
        bs    = border_style,
        ov    = outline_val,
        align = alignment,
        mv    = margin_v,
    );

    for block in blocks {
        let body = if s.karaoke_enabled && !block.words.is_empty() && !block.edited {
            block.words.iter().map(|w| {
                let dur_cs = ((w.end - w.start) * 100.0).round() as u64;
                format!("{{\\k{}}}{} ", dur_cs, w.word.trim())
            }).collect::<String>().trim().to_string()
        } else {
            block.text.trim().to_string()
        };
        let fx = match s.animation.as_str() {
            "fade" => "{\\fad(150,150)}",
            "pop"  => "{\\t(0,100,\\fscx115\\fscy115)\\t(100,200,\\fscx100\\fscy100)}",
            _      => "",
        };
        out.push_str(&format!(
            "Dialogue: 0,{start},{end},Default,,0,0,{mv},,{fx}{body}\n",
            start = secs_to_ass(block.start),
            end   = secs_to_ass(block.end),
            mv    = margin_v,
            fx    = fx,
            body  = body,
        ));
    }
    out
}

// ── Export SRT ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ExportSrtArgs {
    pub project_id: String,
    pub blocks:     Vec<SubtitleBlock>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSrtResult {
    pub success: bool, pub output_path: String, pub error: Option<String>,
}

#[tauri::command]
pub fn export_srt(args: ExportSrtArgs) -> Result<ExportSrtResult, String> {
    let path = output_path(&args.project_id, "srt")?;
    let content = args.blocks.iter().enumerate().map(|(i, b)| {
        format!("{}\n{} --> {}\n{}\n\n", i+1, secs_to_srt(b.start), secs_to_srt(b.end), b.text.trim())
    }).collect::<String>();
    fs::write(&path, content).map_err(|e| format!("Écriture SRT: {}", e))?;
    Ok(ExportSrtResult { success: true, output_path: path.to_string_lossy().to_string(), error: None })
}

// ── Export ASS ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ExportAssArgs {
    pub project_id: String,
    pub blocks:     Vec<SubtitleBlock>,
    pub style:      SubtitleStyle,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportAssResult {
    pub success: bool, pub output_path: String, pub error: Option<String>,
}

#[tauri::command]
pub fn export_ass(args: ExportAssArgs) -> Result<ExportAssResult, String> {
    let path = output_path(&args.project_id, "ass")?;
    fs::write(&path, build_ass(&args.blocks, &args.style, 1920, 1080))
        .map_err(|e| format!("Écriture ASS: {}", e))?;
    Ok(ExportAssResult { success: true, output_path: path.to_string_lossy().to_string(), error: None })
}

// ── Burn-in FFmpeg ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BurnSubtitlesArgs {
    pub project_id:    String,
    pub video_path:    String,
    pub blocks:        Vec<SubtitleBlock>,
    pub style:         SubtitleStyle,
    /// Segments keep (start_time, end_time) en secondes.
    /// Si vide : la vidéo est déjà trimée, on burne directement.
    /// Si fourni : Rust fait trim + burn en une seule passe FFmpeg.
    #[serde(default)]
    pub keep_segments: Vec<[f64; 2]>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BurnSubtitlesResult {
    pub success: bool, pub output_path: String, pub error: Option<String>,
}

#[tauri::command]
pub fn burn_subtitles(args: BurnSubtitlesArgs) -> Result<BurnSubtitlesResult, String> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

    // Résolution réelle pour PlayResX/Y
    let (vid_w, vid_h) = probe_video_dimensions(&args.video_path);

    // Normaliser keep_segments : Vec<[f64;2]> → Vec<(f64,f64)> puis merge
    let keeps: Vec<(f64,f64)> = merge_keeps(
        &args.keep_segments.iter().map(|s| (s[0], s[1])).collect::<Vec<_>>()
    );

    // Remappe les blocs si keep_segments fournis
    // (les blocs arrivent avec timestamps originaux de la vidéo source)
    let remapped_blocks: Vec<SubtitleBlock> = if keeps.is_empty() {
        args.blocks.clone()
    } else {
        args.blocks.iter().map(|b| {
            let new_start = remap_time(b.start, &keeps);
            let new_end   = remap_time(b.end,   &keeps);
            let new_words = b.words.iter().map(|w| SubtitleWord {
                word:  w.word.clone(),
                start: remap_time(w.start, &keeps),
                end:   remap_time(w.end,   &keeps),
            }).collect();
            SubtitleBlock {
                start:  new_start,
                end:    new_end,
                text:   b.text.clone(),
                words:  new_words,
                edited: b.edited,
            }
        }).collect()
    };

    // Générer le fichier ASS avec timestamps remappés
    let ass_path = std::env::temp_dir().join(format!("autotrim_{}.ass", ts));
    fs::write(&ass_path, build_ass(&remapped_blocks, &args.style, vid_w, vid_h))
        .map_err(|e| format!("Écriture ASS temp: {}", e))?;

    let out = output_path(&args.project_id, "mp4")?;

    // Échappement chemin ASS (Windows : \ → /, : → \:)
    let ass_str = ass_path.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    let ass_esc = ass_str.replace('\\', "/").replace(':', "\\:");
    #[cfg(not(target_os = "windows"))]
    let ass_esc = ass_str.clone();

    // Construire la commande FFmpeg selon qu'on a des segments ou non
    let mut cmd = std::process::Command::new("ffmpeg");
    cmd.arg("-y").arg("-i").arg(&args.video_path);

    let result = if keeps.is_empty() {
        // Vidéo déjà trimée → juste burn ASS
        cmd.args([
            "-vf",     &format!("ass='{}'", ass_esc),
            "-c:v",    "libx264",
            "-preset", "fast",
            "-crf",    "18",
            "-c:a",    "copy",
            &out.to_string_lossy(),
        ]).output()
    } else {
        // Trim + burn ASS en une seule passe
        let n = keeps.len();
        let mut filter_parts: Vec<String> = Vec::new();
        for (i, (s, e)) in keeps.iter().enumerate() {
            filter_parts.push(format!(
                "[0:v]trim=start={s:.4}:end={e:.4},setpts=PTS-STARTPTS[v{i}]",
                s=s, e=e, i=i
            ));
            filter_parts.push(format!(
                "[0:a]atrim=start={s:.4}:end={e:.4},asetpts=PTS-STARTPTS[a{i}]",
                s=s, e=e, i=i
            ));
        }
        let v_concat: String = (0..n).map(|i| format!("[v{}]", i)).collect();
        let a_concat: String = (0..n).map(|i| format!("[a{}]", i)).collect();
        filter_parts.push(format!(
            "{v}concat=n={n}:v=1:a=0[vtrim]",
            v=v_concat, n=n
        ));
        filter_parts.push(format!(
            "{a}concat=n={n}:v=0:a=1[atrim]",
            a=a_concat, n=n
        ));
        filter_parts.push(format!("[vtrim]ass='{}'[vout]", ass_esc));

        let filter_complex = filter_parts.join(";");
        cmd.args([
            "-filter_complex", &filter_complex,
            "-map",    "[vout]",
            "-map",    "[atrim]",
            "-c:v",    "libx264",
            "-preset", "fast",
            "-crf",    "18",
            "-c:a",    "aac",
            "-b:a",    "192k",
            &out.to_string_lossy(),
        ]).output()
    }.map_err(|e| format!("ffmpeg introuvable: {}", e))?;

    let _ = fs::remove_file(&ass_path);

    if !result.status.success() {
        let err = String::from_utf8_lossy(&result.stderr);
        return Ok(BurnSubtitlesResult {
            success: false,
            output_path: String::new(),
            error: Some(format!("FFmpeg: {}", &err[..err.len().min(800)])),
        });
    }
    Ok(BurnSubtitlesResult {
        success: true,
        output_path: out.to_string_lossy().to_string(),
        error: None,
    })
}