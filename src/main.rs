use mjrs::{Grid, Symmetries, cells, markov};
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
    markov!(Cell, {
        #[Symmetries::NONE]
        rule seed = [[Black -> White]];
        grid.apply_once(&seed);

        rule seed = [[Black -> Red]];
        grid.apply_once(&seed);

        #[Symmetries::ROTATIONS]
        rule spread = [[Red, Black -> Red]] | [[White, Black -> White]];
        while grid.apply_once(&spread) {}

        rule border = [[Red -> Blue, White -> Blue]];
        grid.apply_all(&border);

        rule clear = [[Red | White -> Black]];
        grid.apply_all(&clear);

        rule spread = [[Blue, Black -> Blue]];
        grid.apply_all(&spread);
        rule spread = [
            [Black -> Blue, Blue]
            [Blue, Black]
        ];
        while grid.apply_all(&spread) {}

        rule seed = [[Blue, Black -> Green]];
        while grid.apply_all(&seed) {}

        rule seed = [[Black -> DarkGreen]];
        for _ in 0..13 {
            grid.apply_once(&seed);
        }

        rule spread_green = [[Green, Black -> Green]] | [[DarkGreen, Black -> DarkGreen]];
        while grid.apply_once(&spread_green) {}
    });
    grid.export().unwrap().save("output.png").unwrap();
}
