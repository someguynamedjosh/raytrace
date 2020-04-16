use byteorder::{LittleEndian, WriteBytesExt};
use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::PathBuf;

use super::{PackedChunk, PackedChunkData, UnpackedChunkData, CHUNK_VOLUME};

pub type ChunkStorageCoord = (isize, isize, isize, u8);

const HEADER_SIZE: u64 = 16;
const MATERIAL_SIZE: u64 = CHUNK_VOLUME as u64 * 4; // u32 for each material.
const MINEFIELD_SIZE: u64 = CHUNK_VOLUME as u64; // u8 for each cell.

struct ChunkStorage {
    storage_dir: PathBuf,
}

impl ChunkStorage {
    pub fn new() -> ChunkStorage {
        let storage_dir = dirs::config_dir()
            .expect("System somehow doesn't have a config dir?")
            .join("raytrace")
            .join("world");
        ChunkStorage { storage_dir }
    }

    fn get_path_for(&self, coord: &ChunkStorageCoord) -> PathBuf {
        let filename = format!(
            "{:08X}{:08X}{:08X}{:02X}",
            coord.0, coord.1, coord.2, coord.3
        );
        self.storage_dir.join(filename)
    }

    fn create_empty_chunk_file(&self, coord: &ChunkStorageCoord) -> File {
        let filename = self.get_path_for(coord);
        let file = File::create(&filename).expect("Cannot create chunk storage");
        file.set_len(HEADER_SIZE + MATERIAL_SIZE + MINEFIELD_SIZE)
            .expect("Cannot resize chunk storage");
        file
    }

    fn write_chunk_data(file: &mut File, data: &PackedChunkData) -> io::Result<()> {
        file.seek(SeekFrom::Start(HEADER_SIZE))?;
        for piece in data.borrow_materials() {
            file.write(&piece.to_le_bytes())?;
        }
        file.write_all(data.borrow_minefield())?;
        Ok(())
    }

    fn read_into_packed_chunk_data(file: &mut File, data: &mut PackedChunkData) -> io::Result<()> {
        file.seek(SeekFrom::Start(HEADER_SIZE))?;
        Ok(())
    }

    fn has_chunk(&self, coord: &ChunkStorageCoord) -> bool {
        self.get_path_for(coord).exists()
    }

    pub fn get_chunk(&self, coord: &ChunkStorageCoord) -> UnpackedChunkData {
        unimplemented!()
    }

    pub fn get_packed_chunk(&self, coord: &ChunkStorageCoord) -> PackedChunk {
        unimplemented!()
    }
}
