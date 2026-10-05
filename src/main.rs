use mjrs::{Grid, Symmetries, cells, rule_set};
use rgb::RGB8;

cells! {
    Cell {
        Black => RGB8::new(0, 0, 0),
        White => RGB8::new(255, 255, 255),
        Red => RGB8::new(255, 0, 0),
        Blue => RGB8::new(89, 146, 240),
        Green => RGB8::new(0, 255, 0),
        DarkGreen => RGB8::new(20, 153, 69)
    }
}

fn main() {
    let mut grid = Grid::new(100, 100);
    let seed = rule_set!(Cell, Symmetries::NONE, [[Black -> White]]);
    grid.apply_once(&seed);
    let seed = rule_set!(Cell, Symmetries::NONE, [[Black -> Red]]);
    grid.apply_once(&seed);
    let spread = rule_set!(Cell, Symmetries::ROTATIONS,
        & [[Red, Black -> Red]]
        & [[White, Black -> White]]
    );
    while grid.apply_once(&spread) {}
    let border = rule_set!(Cell, Symmetries::ROTATIONS, [
        [Red -> Blue, White -> Blue]
    ]);
    grid.apply_all(&border);
    let clear = rule_set!(Cell, Symmetries::NONE, [
        [[Red White] -> Black]
    ]);
    grid.apply_all(&clear);
    let spread_blue = rule_set!(Cell, Symmetries::ROTATIONS, [
        [Blue, Black -> Blue]
    ]);
    grid.apply_all(&spread_blue);
    let spread_blue = rule_set!(Cell, Symmetries::ROTATIONS, [
        [Black -> Blue, Blue]
        [Blue, Black]
    ]);
    while grid.apply_all(&spread_blue) {}
    let seed = rule_set!(Cell, Symmetries::ROTATIONS, [
        [Blue, Black -> Green]
    ]);
    while grid.apply_all(&seed) {}
    let seed = rule_set!(Cell, Symmetries::ROTATIONS, [
        [Black -> DarkGreen]
    ]);
    for _ in 0..13 {
        grid.apply_once(&seed);
    }
    let spread = rule_set!(Cell, Symmetries::ROTATIONS, [
        [DarkGreen, Black -> DarkGreen]
    ] & [
        [Green, Black -> Green]
    ]);
    while grid.apply_once(&spread) {}
    grid.export().unwrap().save("output.png").unwrap();
}
