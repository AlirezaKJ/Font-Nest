use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use serde::Serialize;
use serde_json::{Value, json};
use ttf_parser::{Face, GlyphId, OutlineBuilder, Rect, Tag, name_id};

use crate::dto::{
    FontEmbeddingProperties, FontFaceInspection, FontFaceMetrics, FontFaceNames,
    FontFaceProperties, FontGlyphBounds, FontGlyphOutline, FontGlyphOutlineHandle,
    FontGlyphOutlinePoint, FontGlyphVariationValue, FontParserJsonExport, FontVariationAxis,
};

pub const PARSER_NAME: &str = "ttf-parser";
pub const PARSER_VERSION: &str = "0.25.1";

#[derive(Debug, thiserror::Error)]
pub enum FontInspectionError {
    #[error("the font face could not be parsed")]
    Parse,
    #[error("the parser snapshot could not be serialized")]
    Serialize(#[from] serde_json::Error),
    #[error("the requested Unicode codepoint is invalid")]
    InvalidCodepoint,
    #[error("the selected font does not map that character to a glyph")]
    MissingGlyph,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TableRecord {
    tag: String,
    checksum: u32,
    offset: u32,
    length: u32,
}

pub fn inspect_face(
    face_id: &str,
    data: &[u8],
    face_index: u32,
) -> Result<FontFaceInspection, FontInspectionError> {
    let face = Face::parse(data, face_index).map_err(|_| FontInspectionError::Parse)?;
    let codepoints = unicode_codepoints(&face);
    let table_count = table_directory(data, face_index)
        .and_then(|tables| u16::try_from(tables.len()).ok())
        .ok_or(FontInspectionError::Parse)?;
    Ok(FontFaceInspection {
        face_id: face_id.to_owned(),
        parser_name: PARSER_NAME,
        parser_version: PARSER_VERSION,
        metrics: metrics(&face),
        names: face_names(&face),
        properties: face_properties_summary(&face, codepoints.len(), table_count),
        variation_axes: variation_axes(&face),
        unicode_codepoints: codepoints,
    })
}

pub fn export_face_json(
    face_id: &str,
    data: &[u8],
    face_index: u32,
) -> Result<FontParserJsonExport, FontInspectionError> {
    let face = Face::parse(data, face_index).map_err(|_| FontInspectionError::Parse)?;
    let codepoints = unicode_codepoints(&face);
    let glyph_codepoints = glyph_codepoints(&face, &codepoints);
    let tables = table_directory(data, face_index).ok_or(FontInspectionError::Parse)?;
    let snapshot = json!({
        "parser": {
            "name": PARSER_NAME,
            "version": PARSER_VERSION,
        },
        "faceIndex": face_index,
        "dataLength": data.len(),
        "properties": face_properties(&face),
        "metrics": metrics_json(&face),
        "names": names_json(&face),
        "variationAxes": variation_axes_json(&face),
        "tables": tables,
        "unicode": {
            "codepointCount": codepoints.len(),
            "ranges": codepoint_ranges(&codepoints),
            "mappings": unicode_mappings_json(&face, &codepoints),
        },
        "glyphs": glyphs_json(&face, &glyph_codepoints),
        "notes": [
            "Values are a structured snapshot of ttf-parser output.",
            "Binary table payloads and glyph outlines are intentionally omitted; table offsets and lengths refer to the source font data.",
        ],
    });
    let raw_json = serde_json::to_string_pretty(&snapshot)?;
    let json_byte_length = u32::try_from(raw_json.len()).unwrap_or(u32::MAX);

    Ok(FontParserJsonExport {
        face_id: face_id.to_owned(),
        parser_name: PARSER_NAME,
        parser_version: PARSER_VERSION,
        json_byte_length,
        raw_json,
    })
}

pub fn inspect_glyph_outline(
    face_id: &str,
    data: &[u8],
    face_index: u32,
    codepoint: u32,
    variations: &[FontGlyphVariationValue],
) -> Result<FontGlyphOutline, FontInspectionError> {
    let character = char::from_u32(codepoint).ok_or(FontInspectionError::InvalidCodepoint)?;
    let mut face = Face::parse(data, face_index).map_err(|_| FontInspectionError::Parse)?;
    apply_variations(&mut face, variations);
    let glyph = face
        .glyph_index(character)
        .ok_or(FontInspectionError::MissingGlyph)?;
    let mut builder = GlyphOutlineBuilder::default();
    let bounds = face.outline_glyph(glyph, &mut builder).map(glyph_bounds);
    let outline_available = !builder.path_data.is_empty();

    Ok(FontGlyphOutline {
        face_id: face_id.to_owned(),
        codepoint,
        glyph_id: glyph.0,
        glyph_name: face.glyph_name(glyph).map(str::to_owned),
        units_per_em: face.units_per_em(),
        advance_width: face.glyph_hor_advance(glyph),
        left_side_bearing: face.glyph_hor_side_bearing(glyph),
        bounds,
        path_data: builder.path_data,
        points: builder.points,
        handles: builder.handles,
        contour_count: builder.contour_count,
        outline_available,
    })
}

fn apply_variations(face: &mut Face<'_>, variations: &[FontGlyphVariationValue]) {
    for variation in variations.iter().take(64) {
        let bytes = variation.tag.as_bytes();
        let Ok(tag_bytes) = <&[u8; 4]>::try_from(bytes) else {
            continue;
        };
        if variation.value.is_finite() {
            let _ = face.set_variation(Tag::from_bytes(tag_bytes), variation.value);
        }
    }
}

#[derive(Default)]
struct GlyphOutlineBuilder {
    path_data: String,
    points: Vec<FontGlyphOutlinePoint>,
    handles: Vec<FontGlyphOutlineHandle>,
    current: Option<(f32, f32)>,
    contour_start: Option<(f32, f32)>,
    contour_count: u16,
}

impl GlyphOutlineBuilder {
    fn point(&mut self, x: f32, y: f32, kind: &'static str) {
        self.points.push(FontGlyphOutlinePoint { x, y, kind });
    }

    fn handle(&mut self, from: (f32, f32), to: (f32, f32)) {
        self.handles.push(FontGlyphOutlineHandle {
            x1: from.0,
            y1: from.1,
            x2: to.0,
            y2: to.1,
        });
    }
}

impl OutlineBuilder for GlyphOutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let _ = write!(self.path_data, "M{x} {y} ");
        self.point(x, y, "on-curve");
        self.current = Some((x, y));
        self.contour_start = Some((x, y));
        self.contour_count = self.contour_count.saturating_add(1);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let _ = write!(self.path_data, "L{x} {y} ");
        self.point(x, y, "on-curve");
        self.current = Some((x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let _ = write!(self.path_data, "Q{x1} {y1} {x} {y} ");
        let control = (x1, y1);
        let end = (x, y);
        if let Some(start) = self.current {
            self.handle(start, control);
        }
        self.handle(control, end);
        self.point(x1, y1, "off-curve");
        self.point(x, y, "on-curve");
        self.current = Some(end);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let _ = write!(self.path_data, "C{x1} {y1} {x2} {y2} {x} {y} ");
        let first_control = (x1, y1);
        let second_control = (x2, y2);
        let end = (x, y);
        if let Some(start) = self.current {
            self.handle(start, first_control);
        }
        self.handle(second_control, end);
        self.point(x1, y1, "off-curve");
        self.point(x2, y2, "off-curve");
        self.point(x, y, "on-curve");
        self.current = Some(end);
    }

    fn close(&mut self) {
        self.path_data.push_str("Z ");
        self.current = self.contour_start;
    }
}

fn glyph_bounds(bounds: Rect) -> FontGlyphBounds {
    FontGlyphBounds {
        x_min: bounds.x_min,
        y_min: bounds.y_min,
        x_max: bounds.x_max,
        y_max: bounds.y_max,
    }
}

fn metrics(face: &Face<'_>) -> FontFaceMetrics {
    let (capital_height, capital_height_source) = height_metric(face, face.capital_height(), 'H');
    let (x_height, x_height_source) = height_metric(face, face.x_height(), 'x');
    FontFaceMetrics {
        units_per_em: face.units_per_em(),
        ascender: face.ascender(),
        capital_height,
        capital_height_source,
        x_height,
        x_height_source,
        baseline: 0,
        descender: face.descender(),
        line_gap: face.line_gap(),
    }
}

fn face_names(face: &Face<'_>) -> FontFaceNames {
    FontFaceNames {
        full_name: preferred_name(face, name_id::FULL_NAME),
        version: preferred_name(face, name_id::VERSION),
        manufacturer: preferred_name(face, name_id::MANUFACTURER),
        designer: preferred_name(face, name_id::DESIGNER),
        description: preferred_name(face, name_id::DESCRIPTION),
        license: preferred_name(face, name_id::LICENSE),
    }
}

fn preferred_name(face: &Face<'_>, name_id: u16) -> Option<String> {
    let mut fallback = None;
    for name in face
        .names()
        .into_iter()
        .filter(|name| name.name_id == name_id)
    {
        let Some(value) = name.to_string().filter(|value| !value.trim().is_empty()) else {
            continue;
        };
        if name.language_id == 0x0409 {
            return Some(value);
        }
        if fallback.is_none() {
            fallback = Some(value);
        }
    }
    fallback
}

fn face_properties_summary(
    face: &Face<'_>,
    unicode_codepoint_count: usize,
    table_count: u16,
) -> FontFaceProperties {
    let mut traits = Vec::new();
    for (enabled, label) in [
        (face.is_regular(), "Regular"),
        (face.is_italic(), "Italic"),
        (face.is_bold(), "Bold"),
        (face.is_oblique(), "Oblique"),
        (face.is_monospaced(), "Monospaced"),
        (face.is_variable(), "Variable"),
    ] {
        if enabled {
            traits.push(label.to_owned());
        }
    }

    FontFaceProperties {
        glyph_count: face.number_of_glyphs(),
        unicode_codepoint_count: u32::try_from(unicode_codepoint_count).unwrap_or(u32::MAX),
        table_count,
        weight: face.weight().to_number(),
        width: face.width().to_number(),
        italic_angle: face.italic_angle(),
        traits,
        embedding: FontEmbeddingProperties {
            permissions: face.permissions().map(|value| format!("{value:?}")),
            subsetting_allowed: face.is_subsetting_allowed(),
            outline_embedding_allowed: face.is_outline_embedding_allowed(),
        },
    }
}

fn variation_axes(face: &Face<'_>) -> Vec<FontVariationAxis> {
    face.variation_axes()
        .into_iter()
        .map(|axis| FontVariationAxis {
            tag: axis.tag.to_string(),
            name_id: axis.name_id,
            minimum: axis.min_value,
            default: axis.def_value,
            maximum: axis.max_value,
            hidden: axis.hidden,
        })
        .collect()
}

fn face_properties(face: &Face<'_>) -> Value {
    json!({
        "glyphCount": face.number_of_glyphs(),
        "isRegular": face.is_regular(),
        "isItalic": face.is_italic(),
        "isBold": face.is_bold(),
        "isOblique": face.is_oblique(),
        "isMonospaced": face.is_monospaced(),
        "isVariable": face.is_variable(),
        "weight": face.weight().to_number(),
        "width": face.width().to_number(),
        "italicAngle": face.italic_angle(),
        "permissions": face.permissions().map(|value| format!("{value:?}")),
        "subsettingAllowed": face.is_subsetting_allowed(),
        "outlineEmbeddingAllowed": face.is_outline_embedding_allowed(),
        "globalBoundingBox": rect_json(face.global_bounding_box()),
    })
}

fn metrics_json(face: &Face<'_>) -> Value {
    let effective_metrics = metrics(face);
    let underline = face.underline_metrics();
    let strikeout = face.strikeout_metrics();
    let subscript = face.subscript_metrics();
    let superscript = face.superscript_metrics();
    json!({
        "unitsPerEm": face.units_per_em(),
        "ascender": face.ascender(),
        "descender": face.descender(),
        "lineGap": face.line_gap(),
        "xHeight": {
            "reported": face.x_height(),
            "effective": effective_metrics.x_height,
            "source": effective_metrics.x_height_source,
        },
        "capitalHeight": {
            "reported": face.capital_height(),
            "effective": effective_metrics.capital_height,
            "source": effective_metrics.capital_height_source,
        },
        "verticalAscender": face.vertical_ascender(),
        "verticalDescender": face.vertical_descender(),
        "verticalLineGap": face.vertical_line_gap(),
        "verticalHeight": face.vertical_height(),
        "underline": underline.map(|value| json!({
            "position": value.position,
            "thickness": value.thickness,
        })),
        "strikeout": strikeout.map(|value| json!({
            "position": value.position,
            "thickness": value.thickness,
        })),
        "subscript": subscript.map(|value| json!({
            "xSize": value.x_size,
            "ySize": value.y_size,
            "xOffset": value.x_offset,
            "yOffset": value.y_offset,
        })),
        "superscript": superscript.map(|value| json!({
            "xSize": value.x_size,
            "ySize": value.y_size,
            "xOffset": value.x_offset,
            "yOffset": value.y_offset,
        })),
    })
}

fn height_metric(
    face: &Face<'_>,
    reported: Option<i16>,
    representative: char,
) -> (Option<i16>, &'static str) {
    if reported.is_some() {
        return (reported, "os2");
    }
    let derived = face
        .glyph_index(representative)
        .and_then(|glyph| face.glyph_bounding_box(glyph))
        .map(|bounds| bounds.y_max);
    if derived.is_some() {
        (derived, "glyph-bounds")
    } else {
        (None, "unavailable")
    }
}

fn names_json(face: &Face<'_>) -> Vec<Value> {
    face.names()
        .into_iter()
        .map(|name| {
            json!({
                "nameId": name.name_id,
                "platform": format!("{:?}", name.platform_id),
                "encodingId": name.encoding_id,
                "languageId": name.language_id,
                "language": format!("{:?}", name.language()),
                "value": name.to_string(),
                "rawHex": bytes_to_hex(name.name),
            })
        })
        .collect()
}

fn variation_axes_json(face: &Face<'_>) -> Vec<Value> {
    variation_axes(face)
        .into_iter()
        .map(|axis| {
            json!({
                "tag": axis.tag.clone(),
                "nameId": axis.name_id,
                "minimum": axis.minimum,
                "default": axis.default,
                "maximum": axis.maximum,
                "hidden": axis.hidden,
            })
        })
        .collect()
}

fn unicode_codepoints(face: &Face<'_>) -> Vec<u32> {
    let mut codepoints = BTreeSet::new();
    if let Some(cmap) = face.tables().cmap {
        for subtable in cmap.subtables {
            if !subtable.is_unicode() {
                continue;
            }
            subtable.codepoints(|codepoint| {
                if char::from_u32(codepoint)
                    .and_then(|character| face.glyph_index(character))
                    .is_some()
                {
                    codepoints.insert(codepoint);
                }
            });
        }
    }
    codepoints.into_iter().collect()
}

fn glyph_codepoints(face: &Face<'_>, codepoints: &[u32]) -> BTreeMap<u16, Vec<String>> {
    let mut glyphs = BTreeMap::<u16, Vec<String>>::new();
    for codepoint in codepoints {
        let Some(character) = char::from_u32(*codepoint) else {
            continue;
        };
        let Some(glyph) = face.glyph_index(character) else {
            continue;
        };
        glyphs
            .entry(glyph.0)
            .or_default()
            .push(format_codepoint(*codepoint));
    }
    glyphs
}

fn unicode_mappings_json(face: &Face<'_>, codepoints: &[u32]) -> Vec<Value> {
    codepoints
        .iter()
        .filter_map(|codepoint| {
            let character = char::from_u32(*codepoint)?;
            let glyph = face.glyph_index(character)?;
            Some(json!({
                "codepoint": format_codepoint(*codepoint),
                "character": character.to_string(),
                "glyphId": glyph.0,
            }))
        })
        .collect()
}

fn glyphs_json(face: &Face<'_>, codepoints: &BTreeMap<u16, Vec<String>>) -> Vec<Value> {
    (0..face.number_of_glyphs())
        .map(|index| {
            let glyph = GlyphId(index);
            json!({
                "id": index,
                "name": face.glyph_name(glyph),
                "codepoints": codepoints.get(&index).cloned().unwrap_or_default(),
                "horizontalAdvance": face.glyph_hor_advance(glyph),
                "horizontalSideBearing": face.glyph_hor_side_bearing(glyph),
                "verticalAdvance": face.glyph_ver_advance(glyph),
                "verticalSideBearing": face.glyph_ver_side_bearing(glyph),
                "boundingBox": face.glyph_bounding_box(glyph).map(rect_json),
            })
        })
        .collect()
}

fn rect_json(rect: Rect) -> Value {
    json!({
        "xMin": rect.x_min,
        "yMin": rect.y_min,
        "xMax": rect.x_max,
        "yMax": rect.y_max,
        "width": rect.width(),
        "height": rect.height(),
    })
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02X}");
    }
    output
}

fn table_directory(data: &[u8], face_index: u32) -> Option<Vec<TableRecord>> {
    let face_offset = sfnt_face_offset(data, face_index)?;
    let table_count = usize::from(read_u16(data, face_offset.checked_add(4)?)?);
    let records_offset = face_offset.checked_add(12)?;
    let records_length = table_count.checked_mul(16)?;
    records_offset
        .checked_add(records_length)
        .filter(|end| *end <= data.len())?;

    let mut records = Vec::with_capacity(table_count);
    for index in 0..table_count {
        let offset = records_offset.checked_add(index.checked_mul(16)?)?;
        let tag_bytes: [u8; 4] = data.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
        let checksum = read_u32(data, offset.checked_add(4)?)?;
        let table_offset = read_u32(data, offset.checked_add(8)?)?;
        let length = read_u32(data, offset.checked_add(12)?)?;
        let table_end = usize::try_from(table_offset)
            .ok()?
            .checked_add(usize::try_from(length).ok()?)?;
        if table_end > data.len() {
            return None;
        }
        records.push(TableRecord {
            tag: String::from_utf8_lossy(&tag_bytes).into_owned(),
            checksum,
            offset: table_offset,
            length,
        });
    }
    records.sort_by(|left, right| left.tag.cmp(&right.tag));
    Some(records)
}

fn sfnt_face_offset(data: &[u8], face_index: u32) -> Option<usize> {
    if data.get(0..4)? == b"ttcf" {
        let face_count = read_u32(data, 8)?;
        if face_index >= face_count {
            return None;
        }
        let index_offset = usize::try_from(face_index).ok()?.checked_mul(4)?;
        let offset = 12usize.checked_add(index_offset)?;
        usize::try_from(read_u32(data, offset)?).ok()
    } else if face_index == 0 {
        Some(0)
    } else {
        None
    }
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        data.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        data.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn codepoint_ranges(codepoints: &[u32]) -> Vec<String> {
    let Some((&first, remaining)) = codepoints.split_first() else {
        return Vec::new();
    };
    let mut ranges = Vec::new();
    let mut start = first;
    let mut end = first;

    for &codepoint in remaining {
        if codepoint == end.saturating_add(1) {
            end = codepoint;
            continue;
        }
        ranges.push(format_range(start, end));
        start = codepoint;
        end = codepoint;
    }
    ranges.push(format_range(start, end));
    ranges
}

fn format_range(start: u32, end: u32) -> String {
    if start == end {
        format_codepoint(start)
    } else {
        format!("{}-{}", format_codepoint(start), format_codepoint(end))
    }
}

fn format_codepoint(codepoint: u32) -> String {
    if codepoint <= 0xffff {
        format!("U+{codepoint:04X}")
    } else {
        format!("U+{codepoint:06X}")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        codepoint_ranges, export_face_json, inspect_face, inspect_glyph_outline, table_directory,
    };

    #[test]
    fn table_directory_exports_font_records_without_binary_table_data() {
        let mut font = vec![
            0x00, 0x01, 0x00, 0x00, // sfnt version
            0x00, 0x01, // one table
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // search fields
            b'n', b'a', b'm', b'e', // tag
            0x01, 0x02, 0x03, 0x04, // checksum
            0x00, 0x00, 0x00, 0x1c, // offset
            0x00, 0x00, 0x00, 0x04, // length
        ];
        font.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);

        let records = table_directory(&font, 0).expect("a valid table directory");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].tag, "name");
        assert_eq!(records[0].checksum, 0x0102_0304);
        assert_eq!(records[0].offset, 28);
        assert_eq!(records[0].length, 4);
    }

    #[test]
    fn codepoint_ranges_collapse_adjacent_values() {
        assert_eq!(
            codepoint_ranges(&[0x20, 0x21, 0x22, 0x41, 0x42, 0x1f600]),
            ["U+0020-U+0022", "U+0041-U+0042", "U+01F600"]
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn installed_true_type_face_exports_metrics_and_parser_json() {
        let data = std::fs::read(r"C:\Windows\Fonts\arial.ttf")
            .expect("Arial is part of the supported Windows font set");

        let inspection = inspect_face("face:test", &data, 0).expect("font metrics");
        let outline = inspect_glyph_outline("face:test", &data, 0, u32::from('B'), &[])
            .expect("glyph outline");
        let exported = export_face_json("face:test", &data, 0).expect("parser JSON");
        let snapshot: serde_json::Value =
            serde_json::from_str(&exported.raw_json).expect("valid JSON");

        assert!(inspection.metrics.units_per_em > 0);
        assert!(inspection.metrics.capital_height.is_some());
        assert!(inspection.properties.glyph_count > 0);
        assert!(inspection.properties.unicode_codepoint_count > 0);
        assert_eq!(
            inspection.unicode_codepoints.len(),
            inspection.properties.unicode_codepoint_count as usize
        );
        assert!(inspection.properties.table_count > 0);
        assert!(inspection.names.full_name.is_some());
        assert!(outline.outline_available);
        assert!(!outline.path_data.is_empty());
        assert!(outline.contour_count > 0);
        assert!(!outline.points.is_empty());
        assert!(!outline.handles.is_empty());
        assert!(outline.bounds.is_some());
        assert!(
            snapshot["tables"]
                .as_array()
                .is_some_and(|tables| !tables.is_empty())
        );
        assert!(
            snapshot["glyphs"]
                .as_array()
                .is_some_and(|glyphs| !glyphs.is_empty())
        );
        assert!(
            snapshot["unicode"]["codepointCount"]
                .as_u64()
                .is_some_and(|count| count > 0)
        );
        assert_eq!(exported.json_byte_length as usize, exported.raw_json.len());
    }
}
