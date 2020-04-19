use super::{Heightmap, PackedChunkData, UnpackedChunkData, CHUNK_VOLUME};
use crate::render::Material;
use crate::util;
use array_macro::array;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

pub type ChunkStorageCoord = (isize, isize, isize, u8);

const HEADER_SIZE: u64 = 16;
const MATERIAL_SIZE: u64 = CHUNK_VOLUME as u64 * 4; // u32 for each material.
const MINEFIELD_SIZE: u64 = CHUNK_VOLUME as u64; // u8 for each cell.
const UP_MATERIAL_SIZE: u64 = CHUNK_VOLUME as u64 * std::mem::size_of::<Material>() as u64;
const NUM_BUFFERS: usize = 32;

pub struct ChunkStorage {
    storage_dir: PathBuf,
    uc_buffers: [UnpackedChunkData; NUM_BUFFERS],
    uc_buffers_in_use: usize,
    pc_buffers: [PackedChunkData; NUM_BUFFERS],
    pc_buffers_in_use: usize,
}

impl ChunkStorage {
    pub fn new() -> ChunkStorage {
        let storage_dir = dirs::config_dir()
            .expect("System somehow doesn't have a config dir?")
            .join("raytrace")
            .join("world");
        std::fs::create_dir_all(&storage_dir).unwrap();
        ChunkStorage {
            storage_dir,
            uc_buffers: array![UnpackedChunkData::new(0); NUM_BUFFERS],
            uc_buffers_in_use: 0,
            pc_buffers: array![PackedChunkData::new(); NUM_BUFFERS],
            pc_buffers_in_use: 0,
        }
    }

    fn get_path_for(&self, coord: &ChunkStorageCoord) -> PathBuf {
        let filename = format!(
            "{:08X}{:08X}{:08X}{:02X}",
            coord.0, coord.1, coord.2, coord.3
        );
        self.storage_dir.join(filename)
    }

    fn write_packed_chunk_data(file: &mut File, data: &PackedChunkData) -> io::Result<()> {
        file.seek(SeekFrom::Start(HEADER_SIZE))?;
        for piece in &data.materials {
            file.write(&piece.to_le_bytes())?;
        }
        file.write_all(&data.minefield)?;
        Ok(())
    }

    fn write_unpacked_chunk_data(file: &mut File, data: &UnpackedChunkData) -> io::Result<()> {
        file.seek(SeekFrom::Start(
            HEADER_SIZE + MATERIAL_SIZE + MINEFIELD_SIZE,
        ))?;
        for material in &data.materials {
            file.write(&material.albedo.0.to_le_bytes())?;
            file.write(&material.albedo.1.to_le_bytes())?;
            file.write(&material.albedo.2.to_le_bytes())?;
            file.write(&material.emission.0.to_le_bytes())?;
            file.write(&material.emission.1.to_le_bytes())?;
            file.write(&material.emission.2.to_le_bytes())?;
            file.write(&[if material.solid { 1 } else { 0 }])?;
        }
        Ok(())
    }

    fn read_into_packed_chunk_data(file: &mut File, data: &mut PackedChunkData) -> io::Result<()> {
        file.seek(SeekFrom::Start(HEADER_SIZE))?;
        let mut buf = [0; 4];
        for index in 0..CHUNK_VOLUME {
            file.read(&mut buf)?;
            data.materials[index] = u32::from_le_bytes(buf.clone());
        }
        file.read_exact(&mut data.minefield)?;
        Ok(())
    }

    fn read_into_unpacked_chunk_data(
        file: &mut File,
        data: &mut UnpackedChunkData,
    ) -> io::Result<()> {
        file.seek(SeekFrom::Start(
            HEADER_SIZE + MATERIAL_SIZE + MINEFIELD_SIZE,
        ))?;
        for index in 0..CHUNK_VOLUME {
            let mut buf = [0; 13];
            file.read(&mut buf)?;
            data.materials[index].albedo.0 = u16::from_le_bytes([buf[0], buf[1]]);
            data.materials[index].albedo.1 = u16::from_le_bytes([buf[2], buf[3]]);
            data.materials[index].albedo.2 = u16::from_le_bytes([buf[4], buf[5]]);
            data.materials[index].emission.0 = u16::from_le_bytes([buf[6], buf[7]]);
            data.materials[index].emission.1 = u16::from_le_bytes([buf[8], buf[9]]);
            data.materials[index].emission.2 = u16::from_le_bytes([buf[10], buf[11]]);
            data.materials[index].solid = buf[12] > 0;
        }
        Ok(())
    }

    fn has_chunk(&self, coord: &ChunkStorageCoord) -> bool {
        self.get_path_for(coord).exists()
    }

    fn generate_and_store_chunk(&mut self, coord: &ChunkStorageCoord) -> (usize, usize) {
        if coord.3 == 0 {
            let mut heightmap = Heightmap::new();
            super::generate_heightmap(&mut heightmap, &(coord.0, coord.1));
            let unpacked_data = &mut self.uc_buffers[self.uc_buffers_in_use];
            super::generate_chunk(unpacked_data, &(coord.0, coord.1, coord.2), &heightmap);
            let packed_data = &mut self.pc_buffers[self.pc_buffers_in_use];
            unpacked_data.pack_into(packed_data);
            self.store_chunk(
                coord,
                &self.pc_buffers[self.pc_buffers_in_use],
                &self.uc_buffers[self.uc_buffers_in_use],
            );

            self.uc_buffers_in_use += 1;
            self.pc_buffers_in_use += 1;
            (self.uc_buffers_in_use - 1, self.pc_buffers_in_use - 1)
        } else {
            // Reserve buffers for the mip we are about to generate.
            let pc_buffer_index = self.pc_buffers_in_use;
            let uc_buffer_index = self.uc_buffers_in_use;
            self.pc_buffers_in_use += 1;
            self.uc_buffers_in_use += 1;
            let unpacked_data_ptr = self.uc_buffers.as_mut_ptr();

            let mut neighborhood = Vec::with_capacity(8);
            let next_lod_coord = util::scale_signed_coord_3d(&(coord.0, coord.1, coord.2), 2);
            let next_lod = coord.3 - 1;
            for offset in util::coord_iter_3d(2) {
                let coord = util::offset_signed_coord_3d(
                    &next_lod_coord,
                    &util::coord_to_signed_coord(&offset),
                );
                let index = self.load_unpacked_chunk_data(&(coord.0, coord.1, coord.2, next_lod));
                neighborhood.push(index);
            }
            let neighborhood: Vec<_> = neighborhood
                .iter()
                .map(|index| &self.uc_buffers[*index])
                .collect();
            // This is safe because this will always point to a different buffer other than the ones
            // borrowed immutably.
            let unpacked_data = unsafe { &mut *unpacked_data_ptr };
            super::generate_mip(unpacked_data, &neighborhood[..]);
            let packed_data = &mut self.pc_buffers[pc_buffer_index];
            unpacked_data.pack_into(packed_data);
            self.store_chunk(
                coord,
                &self.pc_buffers[pc_buffer_index],
                &self.uc_buffers[uc_buffer_index],
            );
            (pc_buffer_index, uc_buffer_index)
        }
    }

    fn load_chunk_data(&mut self, coord: &ChunkStorageCoord) -> (usize, usize) {
        if self.has_chunk(coord) {
            let mut file =
                File::open(self.get_path_for(coord)).expect("Failed to open chunk storage.");
            Self::read_into_packed_chunk_data(
                &mut file,
                &mut self.pc_buffers[self.pc_buffers_in_use],
            ).unwrap();
            Self::read_into_unpacked_chunk_data(
                &mut file,
                &mut self.uc_buffers[self.uc_buffers_in_use],
            ).unwrap();
            self.pc_buffers_in_use += 1;
            self.uc_buffers_in_use += 1;
            (self.pc_buffers_in_use - 1, self.uc_buffers_in_use - 1)
        } else {
            self.generate_and_store_chunk(coord)
        }
    }

    fn load_packed_chunk_data(&mut self, coord: &ChunkStorageCoord) -> usize {
        if self.has_chunk(coord) {
            let mut file =
                File::open(self.get_path_for(coord)).expect("Failed to open chunk storage.");
            Self::read_into_packed_chunk_data(
                &mut file,
                &mut self.pc_buffers[self.pc_buffers_in_use],
            ).unwrap();
            self.pc_buffers_in_use += 1;
            self.pc_buffers_in_use - 1
        } else {
            let pc_index = self.generate_and_store_chunk(coord).0;
            self.uc_buffers_in_use -= 1;
            pc_index
        }
    }

    fn load_unpacked_chunk_data(&mut self, coord: &ChunkStorageCoord) -> usize {
        if self.has_chunk(coord) {
            let mut file =
                File::open(self.get_path_for(coord)).expect("Failed to open chunk storage.");
            Self::read_into_unpacked_chunk_data(
                &mut file,
                &mut self.uc_buffers[self.uc_buffers_in_use],
            ).unwrap();
            self.uc_buffers_in_use += 1;
            self.uc_buffers_in_use - 1
        } else {
            let uc_index = self.generate_and_store_chunk(coord).1;
            self.pc_buffers_in_use -= 1;
            uc_index
        }
    }

    pub fn borrow_packed_chunk_data(&mut self, coord: &ChunkStorageCoord) -> &PackedChunkData {
        let index = self.load_packed_chunk_data(coord);
        self.pc_buffers_in_use -= 1;
        &self.pc_buffers[index]
    }

    pub fn borrow_unpacked_chunk_data(&mut self, coord: &ChunkStorageCoord) -> &UnpackedChunkData {
        let index = self.load_unpacked_chunk_data(coord);
        self.uc_buffers_in_use -= 1;
        &self.uc_buffers[index]
    }

    pub fn store_chunk(
        &self,
        coord: &ChunkStorageCoord,
        packed_data: &PackedChunkData,
        unpacked_data: &UnpackedChunkData,
    ) {
        let mut file =
            File::create(self.get_path_for(coord)).expect("Failed to create chunk storage.");
        Self::write_packed_chunk_data(&mut file, packed_data)
            .expect("Failed to write to chunk storage.");
        Self::write_unpacked_chunk_data(&mut file, unpacked_data)
            .expect("Failed to write to chunk storage.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    fn make_temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "raytraceTestDir{:08X}",
            rand::thread_rng().next_u32()
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }

    fn cleanup(dir: PathBuf) {
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn save_load() {
        let mut heightmap = Heightmap::new();
        crate::world::generate_heightmap(&mut heightmap, &(0, 0));
        let mut chunk = UnpackedChunkData::new(0);
        crate::world::generate_chunk(&mut chunk, &(0, 0, 0), &heightmap);
        let mut packed = PackedChunkData::new();
        chunk.pack_into(&mut packed);

        let mut storage = ChunkStorage {
            storage_dir: make_temp_dir(),
            ..ChunkStorage::new()
        };
        storage.store_chunk(&(0, 0, 0, 1), &packed, &chunk);
        let retrieved_packed_data = storage.borrow_packed_chunk_data(&(0, 0, 0, 1));
        assert!(retrieved_packed_data == &packed);
        let retrieved_unpacked_data = storage.borrow_unpacked_chunk_data(&(0, 0, 0, 1));
        assert!(retrieved_unpacked_data == &chunk);

        cleanup(storage.storage_dir);
    }

    #[test]
    fn generate() {
        let mut storage = ChunkStorage {
            storage_dir: make_temp_dir(),
            ..ChunkStorage::new()
        };

        storage.borrow_unpacked_chunk_data(&(0, 0, 0, 1));

        cleanup(storage.storage_dir);
    }
}
