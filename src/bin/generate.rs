extern crate raytrace;

use raytrace::util::prelude::*;
use raytrace::*;
use std::io::Write;
use std::time::Instant;

const RADIUS: usize = 32;

struct StatTracker {
    total_items: usize,
    remaining_items: usize,
    total_item_time: f64,
    num_timed_items: f64,
    item_timer: Option<Instant>,
    update_interval: usize,
    update_ticker: usize,
}

impl StatTracker {
    fn new(num_items: usize) -> Self {
        StatTracker {
            total_items: num_items,
            remaining_items: num_items,
            total_item_time: 0.0,
            num_timed_items: 0.0,
            item_timer: None,
            update_interval: 100,
            update_ticker: 0,
        }
    }

    fn start_item(&mut self) {
        self.item_timer = Some(Instant::now());
    }

    fn end_item(&mut self) {
        self.remaining_items -= 1;
        if let Some(timer) = self.item_timer.take() {
            let time = timer.elapsed().as_secs_f64();
            // If it's less than a couple milliseconds, don't bother counting it.
            if time > 0.004 {
                self.total_item_time += time;
                self.num_timed_items += 1.0;
            }
        }
    }

    fn print_status(&mut self) {
        self.update_ticker += 1;
        if self.update_ticker < self.update_interval {
            return;
        }
        self.update_ticker = 0;
        let percent = (self.total_items - self.remaining_items) as f64 / self.total_items as f64;
        print!("\r{:.1}%", percent * 100.0);
        if self.num_timed_items > 50.0 {
            let time_per_item = self.total_item_time / self.num_timed_items;
            let remaining_seconds = (self.remaining_items as f64 * time_per_item) as u64;
            print!(
                ", ETA {:01}m{:01}s",
                remaining_seconds / 60,
                remaining_seconds % 60,
            );
        }
        print!("                    ");
        std::io::stdout().flush().unwrap();
    }
}

fn main() {
    let mut game = game::Game::new();
    let radius = RADIUS;
    let num_items = (radius * 2).pow(3);
    let mut stat_tracker = StatTracker::new(num_items);
    println!("\nGenerating chunks...");
    for coord in util::coord_iter_3d(radius * 2) {
        let world_coord = coord.signed().sub((radius as isize).repeat());
        stat_tracker.start_item();
        game.borrow_world_mut().borrow_packed_chunk_data(&(
            world_coord.0,
            world_coord.1,
            world_coord.2,
        ));
        stat_tracker.end_item();
        stat_tracker.print_status();
    }
    println!("Done!");
}
