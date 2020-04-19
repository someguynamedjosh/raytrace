use super::{Heightmap, PackedChunkData, UnpackedChunkData};
use crate::util;
use array_macro::array;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

pub type ChunkStorageCoord = (isize, isize, isize, u8);

const HEADER_SIZE: u64 = 16;
const NUM_BUFFERS: usize = 32;

pub struct ChunkStorage {
    storage_dir: PathBuf,
    uc_buffers: [UnpackedChunkData; NUM_BUFFERS],
    available_uc_buffers: Vec<usize>,
    pc_buffers: [PackedChunkData; NUM_BUFFERS],
    available_pc_buffers: Vec<usize>,
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
            available_uc_buffers: (0..NUM_BUFFERS).collect(),
            pc_buffers: array![PackedChunkData::new(); NUM_BUFFERS],
            available_pc_buffers: (0..NUM_BUFFERS).collect(),
        }
    }

    fn get_path_for(base: &PathBuf, coord: &ChunkStorageCoord) -> PathBuf {
        let filename = format!(
            "{:016X}{:016X}{:016X}{:02X}",
            coord.0, coord.1, coord.2, coord.3
        );
        base.join(filename)
    }

    fn store_chunk(base_path: &PathBuf, coord: &ChunkStorageCoord, packed_data: &PackedChunkData, scale: u8) {
        let mut file = File::create(Self::get_path_for(base_path, coord))
            .expect("Failed to create chunk storage.");
        Self::write_packed_chunk_data(&mut file, packed_data, scale)
            .expect("Failed to write to chunk storage.");
    }

    fn write_packed_chunk_data(file: &mut File, data: &PackedChunkData, scale: u8) -> io::Result<()> {
        file.seek(SeekFrom::Start(HEADER_SIZE - 1))?;
        file.write_all(&[scale])?;
        unsafe {
            let mat_slice = &data.materials[..];
            let mat_slice_u8 =
                std::slice::from_raw_parts(mat_slice.as_ptr() as *const u8, mat_slice.len() * 4);
            file.write_all(mat_slice_u8)?;
        }
        file.write_all(&data.minefield)?;
        Ok(())
    }

    fn read_into_packed_chunk_data(file: &mut File, data: &mut PackedChunkData) -> io::Result<u8> {
        file.seek(SeekFrom::Start(HEADER_SIZE - 1))?;
        let mut scale = [0; 1];
        file.read_exact(&mut scale)?;
        unsafe {
            let mat_slice = &mut data.materials[..];
            let mat_slice_u8 =
                std::slice::from_raw_parts_mut(mat_slice.as_ptr() as *mut u8, mat_slice.len() * 4);
            file.read_exact(mat_slice_u8)?;
        }
        file.read_exact(&mut data.minefield[..])?;
        Ok(scale[0])
    }

    fn has_chunk(&self, coord: &ChunkStorageCoord) -> bool {
        Self::get_path_for(&self.storage_dir, coord).exists()
    }

    fn generate_and_store_chunk(&mut self, coord: &ChunkStorageCoord) -> (usize, usize) {
        if coord.3 == 0 {
            let pc_buffer_index = self.available_pc_buffers.pop().unwrap();
            let uc_buffer_index = self.available_uc_buffers.pop().unwrap();

            let mut heightmap = Heightmap::new();
            super::generate_heightmap(&mut heightmap, &(coord.0, coord.1));
            let unpacked_data = &mut self.uc_buffers[uc_buffer_index];
            super::generate_chunk(unpacked_data, &(coord.0, coord.1, coord.2), &heightmap);
            let packed_data = &mut self.pc_buffers[pc_buffer_index];
            unpacked_data.pack_into(packed_data);
            Self::store_chunk(
                &self.storage_dir,
                coord,
                &self.pc_buffers[pc_buffer_index],
                unpacked_data.scale,
            );

            (pc_buffer_index, uc_buffer_index)
        } else {
            let pc_buffer_index = self.available_pc_buffers.pop().unwrap();
            let uc_buffer_index = self.available_uc_buffers.pop().unwrap();
            let unpacked_data_ptr = self.uc_buffers.as_mut_ptr();

            let mut neighborhood = Vec::with_capacity(8);
            let next_lod_coord = util::scale_signed_coord_3d(&(coord.0, coord.1, coord.2), 2);
            let next_lod = coord.3 - 1;
            for offset in util::coord_iter_3d(2) {
                let coord = util::offset_signed_coord_3d(
                    &next_lod_coord,
                    &util::coord_to_signed_coord(&offset),
                );
                let (unused, index) = self
                    .load_chunk_data(&(coord.0, coord.1, coord.2, next_lod));
                self.available_pc_buffers.push(unused);
                neighborhood.push(index);
            }
            let neighborhood_refs: Vec<_> = neighborhood
                .iter()
                .map(|index| &self.uc_buffers[*index])
                .collect();
            // This is safe because this will always point to a different buffer than the other ones
            let unpacked_data = unsafe { &mut *unpacked_data_ptr.offset(uc_buffer_index as isize) };
            super::generate_mip(unpacked_data, &neighborhood_refs[..]);
            let packed_data = &mut self.pc_buffers[pc_buffer_index];
            unpacked_data.pack_into(packed_data);
            Self::store_chunk(
                &self.storage_dir,
                coord,
                &self.pc_buffers[pc_buffer_index],
                unpacked_data.scale,
            );
            self.available_uc_buffers.append(&mut neighborhood);
            (pc_buffer_index, uc_buffer_index)
        }
    }

    fn load_chunk_data(&mut self, coord: &ChunkStorageCoord) -> (usize, usize) {
        if self.has_chunk(coord) {
            let pc_buffer_index = self.available_pc_buffers.pop().unwrap();
            let uc_buffer_index = self.available_uc_buffers.pop().unwrap();

            let mut file = File::open(Self::get_path_for(&self.storage_dir, coord))
                .expect("Failed to open chunk storage.");
            let scale = Self::read_into_packed_chunk_data(
                &mut file,
                &mut self.pc_buffers[pc_buffer_index],
            )
            .unwrap();
            self.pc_buffers[pc_buffer_index]
                .unpack_into(&mut self.uc_buffers[uc_buffer_index], scale);
            (pc_buffer_index, uc_buffer_index)
        } else {
            let r = self.generate_and_store_chunk(coord);
            r
        }
    }

    fn load_packed_chunk_data(&mut self, coord: &ChunkStorageCoord) -> usize {
        if self.has_chunk(coord) {
            let pc_buffer_index = self.available_pc_buffers.pop().unwrap();

            let mut file = File::open(Self::get_path_for(&self.storage_dir, coord))
                .expect("Failed to open chunk storage.");
            Self::read_into_packed_chunk_data(
                &mut file,
                &mut self.pc_buffers[pc_buffer_index],
            )
            .unwrap();

            pc_buffer_index
        } else {
            let (pc_index, unused) = self.generate_and_store_chunk(coord);
            self.available_uc_buffers.push(unused);
            pc_index
        }
    }

    pub fn borrow_packed_chunk_data(&mut self, coord: &ChunkStorageCoord) -> &PackedChunkData {
        let index = self.load_packed_chunk_data(coord);
        self.available_pc_buffers.push(index);
        &self.pc_buffers[index]
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

        storage.borrow_packed_chunk_data(&(0, 0, 0, 1));

        cleanup(storage.storage_dir);
    }
}