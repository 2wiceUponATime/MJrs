use mjrs::{Grid, Symmetries, cells, pattern_set};
use rgb::RGB8;

cells! {
    Cell {
        B => RGB8::new(0, 0, 0),
        W => RGB8::new(255, 255, 255),
    }
}

fn main() {
    let grid: Grid<Cell> = Grid::new(10, 10);
    pattern_set!(patterns, Cell, Symmetries::ROTATIONS, [
        [B W _]
        [_ [B W] W]
    ]);
    println!("{grid}");
    println!("{patterns}");
}
