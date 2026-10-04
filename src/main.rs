use mjrs::{Grid, cells};
use rgb::RGB8;

use Cell::*;

cells! {
    Cell {
        Black => RGB8::new(0, 0, 0),
        White => RGB8::new(255, 255, 255),
    }
}

fn main() {
    let mut grid = Grid::new(10, 10);
    grid[(0, 0)] = White;
    println!("{grid}");
}
