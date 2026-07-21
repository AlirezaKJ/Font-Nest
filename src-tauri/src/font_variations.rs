//! Answers one question about a font file on disk: does this face carry variation axes?
//!
//! The catalogue asks this for every face it scans, so reading whole font files is not an
//! option: a library of a few thousand faces would mean hundreds of megabytes of I/O
//! before the first row appears. A face declares its axes in the `fvar` table, and a
//! font's table directory sits at a known offset and lists every table by tag, so the
//! answer is a few kilobytes of reads per face rather than the file itself.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Tables per face are capped so a malformed or hostile directory cannot make `FontNest`
/// read megabytes. Real fonts stay well under this; the largest system fonts use ~30.
const MAX_TABLE_RECORDS: u16 = 512;
/// Faces per collection, capped for the same reason.
const MAX_COLLECTION_FACES: u32 = 4096;

const TABLE_RECORD_SIZE: usize = 16;
const VARIATIONS_TAG: &[u8; 4] = b"fvar";
const COLLECTION_TAG: &[u8; 4] = b"ttcf";

/// True when the face at `face_index` declares variation axes.
///
/// Any unreadable, truncated, or malformed file answers false: an unknown font is
/// reported as static rather than advertised with axes it may not have.
pub fn face_is_variable(path: &Path, face_index: u32) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    read_has_variations(&mut BufReader::new(file), face_index).unwrap_or(false)
}

fn read_has_variations<R: Read + Seek>(reader: &mut R, face_index: u32) -> Option<bool> {
    let directory_offset = face_directory_offset(reader, face_index)?;

    reader
        .seek(SeekFrom::Start(u64::from(directory_offset)))
        .ok()?;
    let mut header = [0_u8; 12];
    reader.read_exact(&mut header).ok()?;

    let table_count = u16::from_be_bytes([header[4], header[5]]);
    if table_count > MAX_TABLE_RECORDS {
        return None;
    }

    let mut records = vec![0_u8; usize::from(table_count) * TABLE_RECORD_SIZE];
    reader.read_exact(&mut records).ok()?;

    Some(
        records
            .chunks_exact(TABLE_RECORD_SIZE)
            .any(|record| &record[..4] == VARIATIONS_TAG),
    )
}

/// Resolves where the requested face's table directory begins. A plain font keeps it at
/// the start of the file; a collection lists one offset per face.
fn face_directory_offset<R: Read + Seek>(reader: &mut R, face_index: u32) -> Option<u32> {
    let mut tag = [0_u8; 4];
    reader.read_exact(&mut tag).ok()?;

    if &tag != COLLECTION_TAG {
        return if face_index == 0 { Some(0) } else { None };
    }

    // Collection header: tag, major/minor version, then the face count and its offsets.
    let mut counts = [0_u8; 8];
    reader.read_exact(&mut counts).ok()?;
    let face_count = u32::from_be_bytes([counts[4], counts[5], counts[6], counts[7]]);
    if face_index >= face_count.min(MAX_COLLECTION_FACES) {
        return None;
    }

    let offset_position = 12_u64 + u64::from(face_index) * 4;
    reader.seek(SeekFrom::Start(offset_position)).ok()?;
    let mut offset = [0_u8; 4];
    reader.read_exact(&mut offset).ok()?;
    Some(u32::from_be_bytes(offset))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::read_has_variations;

    /// Builds a minimal font: an sfnt header followed by table records, no table data.
    fn font(tags: &[&[u8; 4]]) -> Vec<u8> {
        let mut bytes = vec![0x00, 0x01, 0x00, 0x00];
        bytes.extend_from_slice(
            &u16::try_from(tags.len())
                .expect("a small table count")
                .to_be_bytes(),
        );
        bytes.extend_from_slice(&[0; 6]);
        for tag in tags {
            bytes.extend_from_slice(*tag);
            bytes.extend_from_slice(&[0; 12]);
        }
        bytes
    }

    /// Builds a collection whose faces are the given fonts, laid out back to back.
    fn collection(faces: &[Vec<u8>]) -> Vec<u8> {
        let header_length = 12 + faces.len() * 4;
        let mut header = b"ttcf".to_vec();
        header.extend_from_slice(&[0, 1, 0, 0]);
        header.extend_from_slice(
            &u32::try_from(faces.len())
                .expect("a small face count")
                .to_be_bytes(),
        );

        let mut body = Vec::new();
        for face in faces {
            let offset = header_length + body.len();
            header.extend_from_slice(&u32::try_from(offset).expect("a small offset").to_be_bytes());
            body.extend_from_slice(face);
        }
        header.extend_from_slice(&body);
        header
    }

    #[test]
    fn a_face_with_an_fvar_table_is_variable() {
        let bytes = font(&[b"cmap", b"fvar", b"glyf"]);

        assert_eq!(read_has_variations(&mut Cursor::new(bytes), 0), Some(true));
    }

    #[test]
    fn a_face_without_an_fvar_table_is_static() {
        let bytes = font(&[b"cmap", b"glyf", b"head"]);

        assert_eq!(read_has_variations(&mut Cursor::new(bytes), 0), Some(false));
    }

    #[test]
    fn each_face_of_a_collection_is_answered_separately() {
        let bytes = collection(&[font(&[b"cmap", b"glyf"]), font(&[b"cmap", b"fvar"])]);

        assert_eq!(
            read_has_variations(&mut Cursor::new(bytes.clone()), 0),
            Some(false)
        );
        assert_eq!(read_has_variations(&mut Cursor::new(bytes), 1), Some(true));
    }

    #[test]
    fn a_face_index_past_the_end_of_a_collection_has_no_answer() {
        let bytes = collection(&[font(&[b"fvar"])]);

        assert_eq!(read_has_variations(&mut Cursor::new(bytes), 4), None);
    }

    #[test]
    fn a_truncated_file_has_no_answer_rather_than_a_guess() {
        let mut bytes = font(&[b"cmap", b"fvar"]);
        bytes.truncate(20);

        assert_eq!(read_has_variations(&mut Cursor::new(bytes), 0), None);
    }
}
