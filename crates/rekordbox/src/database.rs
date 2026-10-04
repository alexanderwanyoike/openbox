use baken_export::pdb::{
    page::{DataPage, HEAP_START, HeaderStyle, PAGE_LEN, index_page},
    rows,
    string::encode,
};
use openbox_domain::{Folder, Library, Playlist, Track, TrackDetails};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};

#[derive(Debug, Clone)]
pub(super) struct Database {
    pub bytes: Arc<Vec<u8>>,
    tables: Vec<Table>,
}

#[derive(Debug, Clone)]
struct Table {
    kind: u32,
    offset: usize,
    pages: Vec<u32>,
}

impl Database {
    pub fn next_playlist_id(&self) -> Result<u64, String> {
        Ok(self
            .rows(7)?
            .iter()
            .map(|r| word(r, 12))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .unwrap_or(0) as u64
            + 1)
    }

    pub fn with_tracks(
        &self,
        mut added: baken_export::pdb::Export,
        library: &Library,
        artwork_rows: &[(u32, String)],
    ) -> Result<Vec<u8>, String> {
        let maximum = |kind, offset| -> Result<u32, String> {
            Ok(self
                .rows(kind)?
                .iter()
                .map(|r| word(r, offset))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .max()
                .unwrap_or(0))
        };
        let artist = maximum(2, 4)?;
        let album = maximum(3, 12)?;
        let genre = maximum(1, 0)?;
        let label = maximum(4, 0)?;
        let key = maximum(5, 0)?;
        let artwork = maximum(13, 0)?;
        let shift = |id: u32, base: u32| -> Result<u32, String> {
            if id == 0 {
                Ok(0)
            } else {
                id.checked_add(base)
                    .ok_or("Device metadata IDs exhausted".into())
            }
        };
        for t in &mut added.tracks {
            t.artist_id = shift(t.artist_id, artist)?;
            t.composer_id = shift(t.composer_id, artist)?;
            t.remixer_id = shift(t.remixer_id, artist)?;
            t.original_artist_id = shift(t.original_artist_id, artist)?;
            t.album_id = shift(t.album_id, album)?;
            t.genre_id = shift(t.genre_id, genre)?;
            t.label_id = shift(t.label_id, label)?;
            t.key_id = shift(t.key_id, key)?;
            t.artwork_id = shift(t.artwork_id, artwork)?;
        }
        let mut edits: Vec<(u32, Vec<Vec<u8>>, bool)> = vec![
            (0, added.tracks.iter().map(|t| t.encode(0)).collect(), true),
            (
                1,
                added
                    .genres
                    .iter()
                    .map(|(id, n)| Ok(rows::named(shift(*id, genre)?, n)))
                    .collect::<Result<_, String>>()?,
                true,
            ),
            (
                2,
                added
                    .artists
                    .iter()
                    .map(|(id, n)| Ok(rows::artist(0, shift(*id, artist)?, n)))
                    .collect::<Result<_, String>>()?,
                true,
            ),
            (
                3,
                added
                    .albums
                    .iter()
                    .map(|(id, a, n)| Ok(rows::album(0, shift(*id, album)?, shift(*a, artist)?, n)))
                    .collect::<Result<_, String>>()?,
                true,
            ),
            (
                4,
                added
                    .labels
                    .iter()
                    .map(|(id, n)| Ok(rows::named(shift(*id, label)?, n)))
                    .collect::<Result<_, String>>()?,
                true,
            ),
            (
                5,
                added
                    .keys
                    .iter()
                    .map(|(id, n)| Ok(rows::key(shift(*id, key)?, n)))
                    .collect::<Result<_, String>>()?,
                true,
            ),
        ];
        edits.push((
            13,
            artwork_rows
                .iter()
                .map(|(id, path)| Ok(rows::named(shift(*id, artwork)?, path)))
                .collect::<Result<_, String>>()?,
            true,
        ));
        edits.retain(|(_, rows, _)| !rows.is_empty());
        edits.push((7, self.playlist_rows(library)?, false));
        edits.push((8, Self::playlist_entries(library)?, false));
        let mut properties: Vec<_> = self.rows(19)?.into_iter().map(|r| r.to_vec()).collect();
        let mut updated_count = false;
        for row in &mut properties {
            if word(row, 0)? == 0x0280 {
                set_word(
                    row,
                    4,
                    library
                        .tracks()
                        .len()
                        .try_into()
                        .map_err(|_| "Too many USB tracks")?,
                );
                updated_count = true;
            }
        }
        if updated_count {
            edits.push((19, properties, false));
        }
        self.update(&edits)
    }

    pub fn playlist_rows(&self, library: &Library) -> Result<Vec<Vec<u8>>, String> {
        let previous: HashMap<_, _> = self
            .rows(7)?
            .into_iter()
            .map(|row| Ok((word(row, 12)? as u64, row)))
            .collect::<Result<_, String>>()?;
        library
            .nodes()
            .map(|node| {
                let is_folder = library.folder(node.id()).is_some();
                let parent = device_id(node.parent().unwrap_or(0))?;
                if let Some(row) = previous.get(&node.id()) {
                    let mut updated = row.get(..20).ok_or("Truncated playlist node")?.to_vec();
                    updated[0..4].copy_from_slice(&parent.to_le_bytes());
                    updated[8..12].copy_from_slice(&node.sort_order().to_le_bytes());
                    updated.extend(encode(node.name()));
                    Ok(updated)
                } else {
                    Ok(rows::playlist_node(
                        parent,
                        node.sort_order(),
                        device_id(node.id())?,
                        is_folder,
                        node.name(),
                    ))
                }
            })
            .collect()
    }

    pub fn playlist_entries(library: &Library) -> Result<Vec<Vec<u8>>, String> {
        library
            .playlists()
            .iter()
            .flat_map(|p| {
                p.track_ids.iter().enumerate().map(move |(i, id)| {
                    Ok(rows::playlist_entry(
                        i as u32 + 1,
                        device_id(*id)?,
                        device_id(p.id)?,
                    ))
                })
            })
            .collect()
    }

    pub fn with_playlists(&self, library: &Library) -> Result<Vec<u8>, String> {
        self.update(&[
            (7, self.playlist_rows(library)?, false),
            (8, Self::playlist_entries(library)?, false),
        ])
    }

    pub fn update(&self, edits: &[(u32, Vec<Vec<u8>>, bool)]) -> Result<Vec<u8>, String> {
        let mut output = (*self.bytes).clone();
        let mut sequence = word(&output, 20)?;
        for (kind, rows, append) in edits {
            let table = self
                .tables
                .iter()
                .find(|t| t.kind == *kind)
                .ok_or("Device database is missing a required metadata table")?;
            let first = (output.len() / PAGE_LEN) as u32;
            output.extend(vec![0; PAGE_LEN]);
            let mut pages = Vec::new();
            // Copy old data pages rather than reencode rows. Unknown fields and deleted slots survive.
            if *append {
                for index in &table.pages {
                    let page =
                        &self.bytes[*index as usize * PAGE_LEN..(*index as usize + 1) * PAGE_LEN];
                    if page[27] & 0x40 == 0 {
                        pages.push(page.to_vec());
                    }
                }
            }
            let mut page = DataPage::new(0, *kind, 0, HeaderStyle::Normal);
            for row in rows {
                let mut row = row.clone();
                if matches!(kind, 0 | 2 | 3) {
                    row[2..4].copy_from_slice(&((page.rows() as u16) * 0x20).to_le_bytes());
                }
                if !page.try_push(&row) {
                    if page.is_empty() {
                        return Err("A device record is too large".into());
                    }
                    pages.push(page.to_bytes());
                    page = DataPage::new(0, *kind, 0, HeaderStyle::Normal);
                    if matches!(kind, 0 | 2 | 3) {
                        row[2..4].fill(0);
                    }
                    if !page.try_push(&row) {
                        return Err("A device record is too large".into());
                    }
                }
            }
            if !page.is_empty() {
                pages.push(page.to_bytes());
            }
            let empty = first + 1 + pages.len() as u32;
            let last = if pages.is_empty() { first } else { empty - 1 };
            let index = index_page(
                first,
                *kind,
                if pages.is_empty() { empty } else { first + 1 },
                (!pages.is_empty()).then_some(first + 1),
            );
            output[first as usize * PAGE_LEN..(first as usize + 1) * PAGE_LEN]
                .copy_from_slice(&index);
            for (i, mut page) in pages.into_iter().enumerate() {
                sequence = sequence
                    .checked_add(1)
                    .ok_or("Database sequence exhausted")?;
                set_word(&mut page, 4, first + 1 + i as u32);
                set_word(&mut page, 12, first + 2 + i as u32);
                set_word(&mut page, 16, sequence);
                output.extend(page);
            }
            output.extend(vec![0; PAGE_LEN]);
            set_word(&mut output, table.offset + 4, empty);
            set_word(&mut output, table.offset + 8, first);
            set_word(&mut output, table.offset + 12, last);
        }
        let next = (output.len() / PAGE_LEN)
            .try_into()
            .map_err(|_| "Device database is too large")?;
        set_word(&mut output, 12, next);
        set_word(
            &mut output,
            20,
            sequence
                .checked_add(1)
                .ok_or("Database sequence exhausted")?,
        );
        Self::read(output.clone())?;
        Ok(output)
    }

    pub fn read(bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.len() < PAGE_LEN
            || !bytes.len().is_multiple_of(PAGE_LEN)
            || word(&bytes, 0)? != 0
            || word(&bytes, 4)? as usize != PAGE_LEN
        {
            return Err("Unsupported or damaged Rekordbox database".into());
        }
        let count = word(&bytes, 8)? as usize;
        if count > (PAGE_LEN - 28) / 16 {
            return Err("Invalid database table count".into());
        }
        let mut tables = Vec::new();
        let mut kinds = HashSet::new();
        let mut all_pages = HashSet::new();
        for i in 0..count {
            let offset = 28 + i * 16;
            let kind = word(&bytes, offset)?;
            if !kinds.insert(kind) {
                return Err("Duplicate database table".into());
            }
            let mut index = word(&bytes, offset + 8)?;
            let last = word(&bytes, offset + 12)?;
            let mut pages = Vec::new();
            loop {
                if index == 0 || !all_pages.insert(index) {
                    return Err("Invalid or cyclic database page chain".into());
                }
                let page = bytes
                    .get(index as usize * PAGE_LEN..(index as usize + 1) * PAGE_LEN)
                    .ok_or("Database page is outside the file")?;
                if word(page, 0)? != 0 || word(page, 4)? != index || word(page, 8)? != kind {
                    return Err("Database page does not match its table".into());
                }
                pages.push(index);
                if index == last {
                    break;
                }
                index = word(page, 12)?;
            }
            tables.push(Table {
                kind,
                offset,
                pages,
            });
        }
        for required in [0, 7, 8] {
            if !kinds.contains(&required) {
                return Err("Device database is missing a required table".into());
            }
        }
        Ok(Self {
            bytes: Arc::new(bytes),
            tables,
        })
    }

    pub fn rows(&self, kind: u32) -> Result<Vec<&[u8]>, String> {
        let Some(table) = self.tables.iter().find(|t| t.kind == kind) else {
            return Ok(Vec::new());
        };
        let mut result = Vec::new();
        for index in &table.pages {
            let page = &self.bytes[*index as usize * PAGE_LEN..(*index as usize + 1) * PAGE_LEN];
            if page[27] & 0x40 != 0 {
                continue;
            }
            // The packed count includes deleted slots. Their contents must never be parsed.
            let count = half(page, 24)? as usize & 0x1fff;
            let groups = count.div_ceil(16);
            let heap_end = HEAP_START + half(page, 30)? as usize;
            let directory_size = count * 2 + groups * 4;
            if heap_end > PAGE_LEN.saturating_sub(directory_size) {
                return Err("Device rows overlap their directory".into());
            }
            let mut offsets = Vec::new();
            for i in 0..count {
                let end = PAGE_LEN - (i / 16) * 36;
                let present = half(page, end - 4)? & (1 << (i % 16)) != 0;
                if present {
                    let start = HEAP_START + half(page, end - 6 - (i % 16) * 2)? as usize;
                    if start >= heap_end {
                        return Err("Invalid database row offset".into());
                    }
                    offsets.push(start);
                }
            }
            let mut sorted = offsets.clone();
            sorted.sort_unstable();
            if sorted.windows(2).any(|w| w[0] == w[1]) {
                return Err("Duplicate database row offset".into());
            }
            for start in offsets {
                let end = sorted
                    .iter()
                    .copied()
                    .find(|end| *end > start)
                    .unwrap_or(heap_end);
                result.push(&page[start..end]);
            }
        }
        Ok(result)
    }

    pub fn library(&self, root: &Path) -> Result<Library, String> {
        let names = |kind, id_offset, name_offset| -> Result<HashMap<u32, String>, String> {
            self.rows(kind)?
                .into_iter()
                .map(|r| Ok((word(r, id_offset)?, string(r, name_offset)?)))
                .collect()
        };
        let mut artists = HashMap::new();
        for row in self.rows(2)? {
            let offset = if half(row, 0)? == 0x64 {
                half(row, 10)? as usize
            } else {
                *row.get(9).ok_or("Invalid artist row")? as usize
            };
            artists.insert(word(row, 4)?, string(row, offset)?);
        }
        let mut albums = HashMap::new();
        for row in self.rows(3)? {
            albums.insert(
                word(row, 12)?,
                string(row, *row.get(21).ok_or("Invalid album row")? as usize)?,
            );
        }
        let genres = names(1, 0, 4)?;
        let keys = names(5, 0, 8)?;
        let artwork = names(13, 0, 4)?;
        let lookup = |names: &HashMap<u32, String>, id| names.get(&id).cloned().unwrap_or_default();
        let mut tracks = Vec::new();
        for row in self.rows(0)? {
            let text = |i: usize| string(row, half(row, 0x5e + i * 2)? as usize);
            let path = super::safe_path(root, &text(20)?)?;
            tracks.push(Track {
                id: word(row, 72)? as u64,
                title: text(17)?,
                artist: lookup(&artists, word(row, 68)?),
                path: path.clone(),
                details: TrackDetails {
                    album: lookup(&albums, word(row, 64)?),
                    album_artist: openbox_metadata::album_artist(&path),
                    genre: lookup(&genres, word(row, 60)?),
                    key: lookup(&keys, word(row, 32)?),
                    bpm: (word(row, 56)? > 0).then(|| word(row, 56).unwrap() as f64 / 100.0),
                    duration_seconds: half(row, 84)? as u32,
                    composer: lookup(&artists, word(row, 12)?),
                    comment: text(16)?,
                    track_number: word(row, 52)?,
                    disc_number: half(row, 76)? as u32,
                    year: half(row, 80)? as u32,
                    artwork: artwork
                        .get(&word(row, 28)?)
                        .map(|relative| -> Result<std::path::PathBuf, String> {
                            let small = super::safe_path(root, relative)?;
                            let large = small.with_file_name(format!(
                                "{}_m.jpg",
                                small.file_stem().unwrap_or_default().to_string_lossy()
                            ));
                            let relative = large
                                .strip_prefix(root)
                                .map_err(|e| e.to_string())?
                                .to_string_lossy();
                            let large = super::safe_path(root, &relative)?;
                            Ok(if large.is_file() { large } else { small })
                        })
                        .transpose()?,
                    analysis: super::analysis::read(&super::safe_path(root, &text(14)?)?),
                },
            });
        }
        let mut entries: HashMap<u64, Vec<(u32, u64)>> = HashMap::new();
        for row in self.rows(8)? {
            entries
                .entry(word(row, 8)? as u64)
                .or_default()
                .push((word(row, 0)?, word(row, 4)? as u64));
        }
        let mut playlists = Vec::new();
        let mut folders = Vec::new();
        let mut floor = 0;
        for row in self.rows(7)? {
            let id = word(row, 12)? as u64;
            floor = floor.max(id);
            let parent = match word(row, 0)? {
                0 => None,
                id => Some(id as u64),
            };
            let sort_order = word(row, 8)?;
            if word(row, 16)? != 0 {
                folders.push(Folder {
                    id,
                    name: string(row, 20)?,
                    parent,
                    sort_order,
                });
                continue;
            }
            let mut items = entries.remove(&id).unwrap_or_default();
            items.sort_by_key(|(position, _)| *position);
            playlists.push(Playlist {
                id,
                name: string(row, 20)?,
                parent,
                sort_order,
                track_ids: items.into_iter().map(|(_, id)| id).collect(),
            });
        }
        Library::from_parts(tracks, playlists, folders, floor)
    }
}

fn device_id(id: u64) -> Result<u32, String> {
    id.try_into().map_err(|_| "Device ID is too large".into())
}

fn set_word(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn word(bytes: &[u8], offset: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or("Truncated database record")?
            .try_into()
            .unwrap(),
    ))
}

fn half(bytes: &[u8], offset: usize) -> Result<u16, String> {
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..offset + 2)
            .ok_or("Truncated database record")?
            .try_into()
            .unwrap(),
    ))
}

fn string(bytes: &[u8], offset: usize) -> Result<String, String> {
    let bytes = bytes
        .get(offset..)
        .ok_or("Invalid database string offset")?;
    let flag = *bytes.first().ok_or("Missing database string")?;
    if flag & 1 != 0 {
        let len = (flag as usize >> 1)
            .checked_sub(1)
            .ok_or("Invalid short string length")?;
        return String::from_utf8(
            bytes
                .get(1..1 + len)
                .ok_or("Truncated database string")?
                .to_vec(),
        )
        .map_err(|e| e.to_string());
    }
    let len = half(bytes, 1)? as usize;
    let content = bytes.get(4..len).ok_or("Invalid database string length")?;
    match flag {
        0x40 => String::from_utf8(content.to_vec()).map_err(|e| e.to_string()),
        0x90 if content.len().is_multiple_of(2) => {
            let units: Vec<_> = content
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16(&units).map_err(|e| e.to_string())
        }
        _ => Err("Unsupported database string encoding".into()),
    }
}
