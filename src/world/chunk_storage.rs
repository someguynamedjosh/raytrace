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
const MATERIAL_STRUCT_SIZE: usize = std::mem::size_of::<Material>();
const UP_MATERIAL_SIZE: u64 = CHUNK_VOLUME as u64 * MATERIAL_STRUCT_SIZE as u64;
const PACKED_FILE_BUFFER_SIZE: usize = (MATERIAL_SIZE + MINEFIELD_SIZE) as usize;
const UNPACKED_FILE_BUFFER_SIZE: usize = UP_MATERIAL_SIZE as usize;
const NUM_BUFFERS: usize = 32;

pub struct ChunkStorage {
    storage_dir: PathBuf,
    uc_buffers: [UnpackedChunkData; NUM_BUFFERS],
    uc_buffers_in_use: usize,
    pc_buffers: [PackedChunkData; NUM_BUFFERS],
    pc_buffers_in_use: usize,

    packed_file_buffer: Vec<u8>,
    unpacked_file_buffer: Vec<u8>,
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
            packed_file_buffer: vec![0; (MATERIAL_SIZE + MINEFIELD_SIZE) as usize],
            unpacked_file_buffer: vec![0; UP_MATERIAL_SIZE as usize],
        }
    }

    fn get_path_for(base: &PathBuf, coord: &ChunkStorageCoord) -> PathBuf {
        let filename = format!(
            "{:016X}{:016X}{:016X}{:02X}",
            coord.0, coord.1, coord.2, coord.3
        );
        base.join(filename)
    }

    fn store_chunk(
        base_path: &PathBuf,
        coord: &ChunkStorageCoord,
        packed_data: &PackedChunkData,
        packed_file_buffer: &mut [u8],
        unpacked_data: &UnpackedChunkData,
        unpacked_file_buffer: &mut [u8],
    ) {
        let mut file = File::create(Self::get_path_for(base_path, coord))
            .expect("Failed to create chunk storage.");
        Self::write_packed_chunk_data(&mut file, packed_data, packed_file_buffer)
            .expect("Failed to write to chunk storage.");
        Self::write_unpacked_chunk_data(&mut file, unpacked_data, unpacked_file_buffer)
            .expect("Failed to write to chunk storage.");
    }

    fn write_packed_chunk_data(
        file: &mut File,
        data: &PackedChunkData,
        buffer: &mut [u8],
    ) -> io::Result<()> {
        file.seek(SeekFrom::Start(HEADER_SIZE))?;
        unsafe {
            let mat_slice = &data.materials[..];
            let mat_slice_u8 =
                std::slice::from_raw_parts(mat_slice.as_ptr() as *const u8, mat_slice.len() * 4);
            file.write_all(mat_slice_u8)?;
        }
        file.write_all(&data.minefield)?;
        Ok(())
    }

    fn write_unpacked_chunk_data(
        file: &mut File,
        data: &UnpackedChunkData,
        buffer: &mut [u8],
    ) -> io::Result<()> {
        let mut index = 0;
        for material in &data.materials {
            unsafe {
                let elementptr = buffer.as_mut_ptr().offset(index) as *mut Material;
                *elementptr = material.clone();
            }
            index += MATERIAL_STRUCT_SIZE as isize;
        }
        file.seek(SeekFrom::Start(
            HEADER_SIZE + MATERIAL_SIZE + MINEFIELD_SIZE,
        ))?;
        file.write_all(buffer)?;
        Ok(())
    }

    fn read_into_packed_chunk_data(
        file: &mut File,
        data: &mut PackedChunkData,
        buffer: &mut [u8],
    ) -> io::Result<()> {
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
        buffer: &mut [u8],
    ) -> io::Result<()> {
        file.seek(SeekFrom::Start(
            HEADER_SIZE + MATERIAL_SIZE + MINEFIELD_SIZE,
        ))?;
        file.read_exact(buffer)?;
        for index in 0..CHUNK_VOLUME {
            unsafe {
                let elementptr = buffer.as_mut_ptr().offset(index as isize) as *mut Material;
                data.materials[index] = (*elementptr).clone();
            }
        }
        Ok(())
    }

    fn has_chunk(&self, coord: &ChunkStorageCoord) -> bool {
        Self::get_path_for(&self.storage_dir, coord).exists()
    }

    fn generate_and_store_chunk(&mut self, coord: &ChunkStorageCoord) -> (usize, usize) {
        if coord.3 == 0 {
            let mut heightmap = Heightmap::new();
            super::generate_heightmap(&mut heightmap, &(coord.0, coord.1));
            let unpacked_data = &mut self.uc_buffers[self.uc_buffers_in_use];
            super::generate_chunk(unpacked_data, &(coord.0, coord.1, coord.2), &heightmap);
            let packed_data = &mut self.pc_buffers[self.pc_buffers_in_use];
            unpacked_data.pack_into(packed_data);
            Self::store_chunk(
                &self.storage_dir,
                coord,
                &self.pc_buffers[self.pc_buffers_in_use],
                &mut self.packed_file_buffer,
                &self.uc_buffers[self.uc_buffers_in_use],
                &mut self.unpacked_file_buffer,
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
            Self::store_chunk(
                &self.storage_dir,
                coord,
                &self.pc_buffers[self.pc_buffers_in_use],
                &mut self.packed_file_buffer,
                &self.uc_buffers[self.uc_buffers_in_use],
                &mut self.unpacked_file_buffer,
            );
            self.uc_buffers_in_use -= 8;
            (pc_buffer_index, uc_buffer_index)
        }
    }

    fn load_chunk_data(&mut self, coord: &ChunkStorageCoord) -> (usize, usize) {
        if self.has_chunk(coord) {
            let mut file = File::open(Self::get_path_for(&self.storage_dir, coord))
                .expect("Failed to open chunk storage.");
            Self::read_into_packed_chunk_data(
                &mut file,
                &mut self.pc_buffers[self.pc_buffers_in_use],
                &mut self.packed_file_buffer[..],
            )
            .unwrap();
            Self::read_into_unpacked_chunk_data(
                &mut file,
                &mut self.uc_buffers[self.uc_buffers_in_use],
                &mut self.unpacked_file_buffer[..],
            )
            .unwrap();
            self.pc_buffers_in_use += 1;
            self.uc_buffers_in_use += 1;
            (self.pc_buffers_in_use - 1, self.uc_buffers_in_use - 1)
        } else {
            self.generate_and_store_chunk(coord)
        }
    }

    fn load_packed_chunk_data(&mut self, coord: &ChunkStorageCoord) -> usize {
        if self.has_chunk(coord) {
            let mut file = File::open(Self::get_path_for(&self.storage_dir, coord))
                .expect("Failed to open chunk storage.");
            Self::read_into_packed_chunk_data(
                &mut file,
                &mut self.pc_buffers[self.pc_buffers_in_use],
                &mut self.packed_file_buffer[..],
            )
            .unwrap();
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
            let mut file = File::open(Self::get_path_for(&self.storage_dir, coord))
                .expect("Failed to open chunk storage.");
            Self::read_into_unpacked_chunk_data(
                &mut file,
                &mut self.uc_buffers[self.uc_buffers_in_use],
                &mut self.unpacked_file_buffer[..],
            )
            .unwrap();
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
    fn generate() {
        let mut storage = ChunkStorage {
            storage_dir: make_temp_dir(),
            ..ChunkStorage::new()
        };

        storage.borrow_unpacked_chunk_data(&(0, 0, 0, 1));

        cleanup(storage.storage_dir);
    }
}
