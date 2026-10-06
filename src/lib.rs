use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, LowerHex},
    hash::Hash,
    mem,
    ops::{BitAnd, BitOr, Index, Not},
};

use bitflags::bitflags;
use bitvec::{bitvec, vec::BitVec};
use image::RgbImage;
use rand::{random_range, rng, seq::SliceRandom};
pub use rgb;
use rgb::RGB8;

#[doc(hidden)]
pub mod __private {
    use std::marker::PhantomData;

    pub struct Sel<A, B>(PhantomData<(A, B)>);
    pub trait Choose<const C: bool> {
        type Out;
    }
    impl<A, B> Choose<true> for Sel<A, B> {
        type Out = A;
    }
    impl<A, B> Choose<false> for Sel<A, B> {
        type Out = B;
    }
}

#[macro_export]
macro_rules! cells {
    (
        $(#[$cells_meta:meta])*
        $vis:vis $name:ident { $($cell:ident => $color:expr),+ $(,)? }
    ) => {
        $(#[$cells_meta])*
        #[derive(Clone, Copy, Hash, PartialEq, Eq)]
        #[repr(u8)]
        $vis enum $name {
            $($cell,)*
        }

        impl $name {
            pub const VALUES: &[Self] = &[$($name::$cell),+];
            pub const COLORS: &[$crate::rgb::RGB8] = &[$($color),+];
            const COUNT: usize = Self::VALUES.len();
        }

        impl From<$name> for u8 {
            fn from(value: $name) -> Self {
                value as u8
            }
        }

        impl From<u8> for $name {
            fn from(value: u8) -> Self {
                Self::VALUES[value as usize]
            }
        }

        impl From<$name> for $crate::rgb::RGB8 {
            fn from(value: $name) -> Self {
                $name::COLORS[value as usize]
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::VALUES[0]
            }
        }

        impl $crate::Cell for $name {
            const COUNT: usize = Self::COUNT;
            type Bits = <$crate::__private::Sel<
                u8,
                <$crate::__private::Sel<
                    u16,
                    <$crate::__private::Sel<
                        u32,
                        <$crate::__private::Sel<
                            u64,
                            u128
                        > as $crate::__private::Choose<{ Self::COUNT <= 64 }>>::Out
                    > as $crate::__private::Choose<{ Self::COUNT <= 32 }>>::Out
                > as $crate::__private::Choose<{ Self::COUNT <= 16 }>>::Out
            > as $crate::__private::Choose<{ Self::COUNT <= 8 }>>::Out;
        }

        const _: () = assert!($name::COUNT <= 128, "too many cells");
    };
}

#[macro_export]
macro_rules! rule_set {
    ($cell:ident, $symmetries:expr, $(|)? $([
        $([$($($in_item:tt)|+ $(-> $out_item:ident)?),+])+
    ])|+) => {$(({
        #[allow(clippy::unused_unit)]
        const WIDTH: u32 = [$([$($crate::rule_set!(@unit $($in_item)+)),+]),+][0].len() as u32;
        #[allow(clippy::unused_unit)]
        const HEIGHT: u32 = [$($crate::rule_set!(@unit $($($in_item)+)+)),+].len() as u32;
        $crate::RuleSet::<$cell>::new(
            WIDTH, HEIGHT,
            vec![
                $($($crate::rule_set!(@in_item $cell, $($in_item)|+)),*),*
            ],
            vec![
                $($($crate::rule_set!(@out_item $cell, $($out_item)?)),*),*
            ],
            $symmetries
        )
    }))|+};

    (@in_item $cell:ident, _) => {{
        use $crate::Bits;
        <$cell as $crate::Cell>::Bits::ALL
    }};

    (@in_item $cell:ident, $($item:ident)|+) => {{
        use $crate::Bits;
        $(<$cell as $crate::Cell>::Bits::bit($cell::$item.into()))|+
    }};

    (@out_item $cell:ident, $item:ident) => {
        Some($cell::$item)
    };

    (@out_item $cell:ident,) => {
        None
    };

    (@unit $($_:tt)*) => {{
        ()
    }};
}

#[macro_export]
macro_rules! markov {
    ($cell:ident, {
        $($body:tt)*
    }) => {
        $crate::markov!(@body $cell, symmetries, $($body)*)
    };

    // `#[symmetries]`
    (@body $cell:ident, $sym:ident, #[$symmetries:expr] $($rest:tt)*) => {
        let $sym = $symmetries;
        $crate::markov!(@body $cell, $sym, $($rest)*);
    };

    // `let name = rule`
    (@body $cell:ident, $sym:ident, let $name:ident = $(|)? $([$($item:tt)*])|*; $($rest:tt)*) => {
        let $name = $crate::rule_set!($cell, $sym, $([$($item)*])|*);
        $crate::markov!(@body $cell, $sym, $($rest)*);
    };

    // Simple block-like expressions
    (@body $cell:ident, $sym:ident, loop $b:block $($rest:tt)*) => {
        loop $b;
        $crate::markov!(@body $cell, $sym, $($rest)*);
    };
    (@body $cell:ident, $sym:ident, unsafe $b:block $($rest:tt)*) => {
        unsafe $b;
        $crate::markov!(@body $cell, $sym, $($rest)*);
    };
    (@body $cell:ident, $sym:ident, $b:block $($rest:tt)*) => {
        $b;
        $crate::markov!(@body $cell, $sym, $($rest)*);
    };

    // Block-like expressions where there are tokens between the keyword and the block
    (@body $cell:ident, $sym:ident, if $($rest:tt)*) => {
        $crate::markov!(@head $cell, $sym, [if] $($rest)*);
    };
    (@body $cell:ident, $sym:ident, match $($rest:tt)*) => {
        $crate::markov!(@head $cell, $sym, [match] $($rest)*);
    };
    (@body $cell:ident, $sym:ident, while $($rest:tt)*) => {
        $crate::markov!(@head $cell, $sym, [while] $($rest)*);
    };
    (@body $cell:ident, $sym:ident, for $($rest:tt)*) => {
        $crate::markov!(@head $cell, $sym, [for] $($rest)*);
    };

    // Consumes other exprs
    (@body $cell:ident, $sym:ident, $e:expr; $($rest:tt)*) => {
        $e;
        $crate::markov!(@body $cell, $sym, $($rest)*);
    };
    // Allow extra semicolons
    (@body $cell:ident, $sym:ident, ; $($rest:tt)*) => {
        $crate::markov!(@body $cell, $sym, $($rest)*);
    };
    // Base case
    (@body $cell:ident, $sym:ident,) => {};

    // Consumes tokens until a block
    (@head $cell:ident, $sym:ident, [$($acc:tt)+] $b:block $($rest:tt)*) => {
        $crate::markov!(@tail $cell, $sym, [$($acc)+ $b] $($rest)*);
    };
    (@head $cell:ident, $sym:ident, [$($acc:tt)+] $t:tt $($rest:tt)*) => {
        $crate::markov!(@head $cell, $sym, [$($acc)+ $t] $($rest)*);
    };

    // Handles `else` and `else if`
    (@tail $cell:ident, $sym:ident, [$($acc:tt)+] else if $($rest:tt)*) => {
        $crate::markov!(@head $cell, $sym, [$($acc)+ else if] $($rest)*);
    };
    (@tail $cell:ident, $sym:ident, [$($acc:tt)+] else $b:block $($rest:tt)*) => {
        $($acc)+ else $b;
        $crate::markov!(@body $cell, $sym, $($rest)*);
    };
    (@tail $cell:ident, $sym:ident, [$($acc:tt)+] $($rest:tt)*) => {
        $($acc)+;
        $crate::markov!(@body $cell, $sym, $($rest)*);
    };
}

macro_rules! impl_bits {
    ($($ty:ty),*) => {
        $(
            impl Bits for $ty {
                const ALL: Self = Self::MAX;
                fn bit(index: u8) -> Self {
                    1 << index as Self
                }
            }
        )*
    };
}

fn to_index(x: u32, y: u32, width: u32) -> usize {
    x as usize + y as usize * width as usize
}

fn from_index(i: usize, width: u32) -> (u32, u32) {
    (i as u32 % width, i as u32 / width)
}

pub trait Bits:
    Default
    + Copy
    + Eq
    + Hash
    + BitAnd<Output = Self>
    + BitOr<Output = Self>
    + Not<Output = Self>
    + LowerHex
{
    const ALL: Self;
    fn bit(index: u8) -> Self;
}

impl_bits!(u8, u16, u32, u64, u128);

pub trait Cell: Into<u8> + Into<RGB8> + Eq + Hash + Default + Copy {
    const COUNT: usize;
    type Bits: Bits;
}

pub struct GridData<'a, T: Cell> {
    width: u32,
    height: u32,
    data: &'a [T],
    updates: &'a [usize],
}

impl<T: Cell> Index<(u32, u32)> for GridData<'_, T> {
    type Output = T;

    fn index(&self, (x, y): (u32, u32)) -> &Self::Output {
        &self.data[to_index(x, y, self.width)]
    }
}

impl<T: Cell> Index<usize> for GridData<'_, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

pub struct Grid<'a, T: Cell> {
    width: u32,
    height: u32,
    data: Vec<T>,
    updates: Vec<usize>,
    nodes: HashMap<*const RuleSet<T>, RuleNode<'a, T>>,
}

impl<'a, T: Cell> Grid<'a, T> {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![T::default(); width as usize * height as usize],
            updates: vec![],
            nodes: HashMap::new(),
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        debug_assert!(
            x < self.width && y < self.height,
            "position out of range: ({x}, {y}) - grid size is ({}, {})",
            self.width,
            self.height
        );
        to_index(x, y, self.width)
    }

    pub fn data(&self) -> GridData<'_, T> {
        GridData {
            width: self.width,
            height: self.height,
            data: &self.data,
            updates: &self.updates,
        }
    }

    pub fn set(&mut self, x: u32, y: u32, value: T) {
        let index = self.index(x, y);
        if self.data[index] != value {
            self.updates.push(index);
        }
        self.data[index] = value;
    }

    pub fn apply_once(&mut self, set: &'a RuleSet<T>) -> bool {
        let key = set as *const RuleSet<T>;
        if !self.nodes.contains_key(&key) {
            self.nodes.insert(key, RuleNode::new(set, self));
        }
        let node = self.nodes.get_mut(&key).unwrap();
        let data = GridData {
            width: self.width,
            height: self.height,
            data: &self.data,
            updates: &self.updates,
        };
        node.update(&data);
        let Some(m) = node.get_match(&data) else {
            return false;
        };
        self.apply(m);
        true
    }

    pub fn apply_all(&mut self, set: &'a RuleSet<T>) -> bool {
        let key = set as *const RuleSet<T>;
        if !self.nodes.contains_key(&key) {
            self.nodes.insert(key, RuleNode::new(set, self));
        }
        let node = self.nodes.get_mut(&key).unwrap();
        let data = GridData {
            width: self.width,
            height: self.height,
            data: &self.data,
            updates: &self.updates,
        };
        node.update(&data);
        let matches = node.get_all_matches(&data);
        if matches.is_empty() {
            return false;
        }
        for m in matches {
            self.apply(m);
        }
        true
    }

    fn apply(&mut self, (ox, oy, rule): (u32, u32, &Rule<T>)) {
        for (i, &out) in rule.output.iter().enumerate() {
            let Some(out) = out else {
                continue;
            };
            let (dx, dy) = from_index(i, rule.width);
            let (x, y) = (ox + dx, oy + dy);
            self.set(x, y, out);
        }
    }

    pub fn export(&self) -> Option<RgbImage> {
        let mut buf = Vec::with_capacity(self.data.len() * 3);
        for cell in &self.data {
            let color: RGB8 = (*cell).into();
            buf.extend_from_slice(&[color.r, color.g, color.b]);
        }
        RgbImage::from_raw(self.width, self.height, buf)
    }
}

impl<T: Cell> Display for Grid<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, cell) in self.data.iter().enumerate() {
            if i != 0 && i % self.width as usize == 0 {
                writeln!(f)?;
            }
            let value: u8 = (*cell).into();
            write!(f, " {value:02x}")?;
        }
        Ok(())
    }
}

impl<T: Cell> Index<(u32, u32)> for Grid<'_, T> {
    type Output = T;

    fn index(&self, index: (u32, u32)) -> &Self::Output {
        &self.data[self.index(index.0, index.1)]
    }
}

impl<T: Cell> Index<usize> for Grid<'_, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

struct Match {
    rule: u32,
    pos: usize,
}

impl Match {
    pub fn new(rule: u32, pos: usize) -> Self {
        Self { rule, pos }
    }
}

struct RuleNode<'a, T: Cell> {
    rules: &'a RuleSet<T>,

    matches: Vec<Match>,
    present: Vec<BitVec>,

    // Index into the grid's update log. None means the rule has never run, so it needs to search
    // the whole grid.
    cursor: Option<usize>,
}

impl<'a, T: Cell> RuleNode<'a, T> {
    pub fn new(rules: &'a RuleSet<T>, grid: &Grid<T>) -> Self {
        let present =
            vec![bitvec![0; grid.width as usize * grid.height as usize]; rules.rules.len()];
        Self {
            rules,
            matches: vec![],
            present,
            cursor: None,
        }
    }

    pub fn get_match(&mut self, grid: &GridData<T>) -> Option<(u32, u32, &'a Rule<T>)> {
        while !self.matches.is_empty() {
            let i = random_range(0..self.matches.len());
            let m = &self.matches[i];
            let (ox, oy) = from_index(m.pos, grid.width);
            let rule = &self.rules.rules[m.rule as usize];
            if rule.matches_at(grid, ox, oy) {
                return Some((ox, oy, rule));
            }
            self.present[m.rule as usize].set(m.pos, false);
            self.matches.swap_remove(i);
        }
        None
    }

    pub fn get_all_matches(&mut self, grid: &GridData<T>) -> Vec<(u32, u32, &'a Rule<T>)> {
        let mut result = vec![];
        let mut indices: Vec<_> = (0..self.matches.len()).collect();
        let mut to_remove = Vec::with_capacity(self.matches.len());
        let mut claimed = bitvec![0; grid.data.len()];
        indices.shuffle(&mut rng());
        for i in indices {
            let m = &self.matches[i];
            let (ox, oy) = from_index(m.pos, grid.width);
            let rule = &self.rules.rules[m.rule as usize];
            if !rule.matches_at(grid, ox, oy) {
                to_remove.push(i);
                continue;
            }
            for x in ox..ox + rule.width {
                for y in oy..oy + rule.height {
                    let j = to_index(x, y, grid.width);
                    if claimed[j] {
                        continue;
                    }
                    claimed.set(j, true);
                }
            }
            result.push((ox, oy, rule));
        }
        to_remove.sort_unstable_by(|a, b| b.cmp(a));
        for i in to_remove {
            self.matches.swap_remove(i);
        }
        result
    }

    pub fn update(&mut self, grid: &GridData<T>) {
        let Some(cursor) = self.cursor else {
            self.cursor = Some(grid.updates.len());
            return self.full_scan(grid);
        };
        self.cursor = Some(grid.updates.len());
        let mut checked = HashSet::new();
        for &pos in &grid.updates[cursor..] {
            let (x, y) = from_index(pos, grid.width);
            let cell: u8 = grid[pos].into();
            for (r, rule) in self.rules.iter().enumerate() {
                if checked.contains(&(pos, r)) {
                    continue;
                }
                checked.insert((pos, r));
                for &(dx, dy) in &rule.matches[cell as usize] {
                    if dx > x || dy > y {
                        continue;
                    }
                    let (ox, oy) = (x - dx, y - dy);
                    let o = to_index(ox, oy, grid.width);
                    if self.present[r][o] {
                        continue;
                    }
                    if rule.matches_at(grid, ox, oy) {
                        self.matches.push(Match::new(r as u32, o));
                        self.present[r].set(o, true);
                    }
                }
            }
        }
    }

    fn full_scan(&mut self, grid: &GridData<T>) {
        self.matches.clear();
        for p in &mut self.present {
            p.fill(false);
        }

        for (r, rule) in self.rules.iter().enumerate() {
            for ly in (rule.height - 1..grid.height).step_by(rule.height as usize) {
                for lx in (rule.width - 1..grid.width).step_by(rule.width as usize) {
                    let cell: u8 = grid[(lx, ly)].into();
                    for (dx, dy) in &rule.matches[cell as usize] {
                        let (ox, oy) = (lx - dx, ly - dy);
                        if ox + rule.width > grid.width || oy + rule.height > grid.height {
                            continue;
                        }
                        if rule.matches_at(grid, ox, oy) {
                            let o = to_index(ox, oy, grid.width);
                            self.present[r].set(o, true);
                            self.matches.push(Match::new(r as u32, o));
                        }
                    }
                }
            }
        }
    }
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct Symmetries: u8 {
        // Shift bits: (horizontal flip, vertical flip, transpose) - transpose is applied first
        const IDENTITY = 1 << 0b000;
        const FLIP_ROT_270 = 1 << 0b001;
        const FLIP_ROT_180 = 1 << 0b010;
        const ROT_270 = 1 << 0b011;
        const FLIP = 1 << 0b100;
        const ROT_90 = 1 << 0b101;
        const ROT_180 = 1 << 0b110;
        const FLIP_ROT_90 = 1 << 0b111;

        const ROTATIONS = Self::ROT_90.bits()
            | Self::ROT_180.bits()
            | Self::ROT_270.bits();
        const NONE = 0;
    }
}

#[derive(PartialEq, Eq)]
struct Rule<T: Cell> {
    width: u32,
    height: u32,
    input: Vec<T::Bits>,
    // None = leave unchanged
    output: Vec<Option<T>>,
    // `matches[i]`: every position within the pattern where cell `i` can be matched
    matches: Vec<Vec<(u32, u32)>>,
}

impl<T: Cell> Rule<T> {
    pub fn new(width: u32, height: u32, input: Vec<T::Bits>, output: Vec<Option<T>>) -> Self {
        let mut matches = vec![Vec::new(); T::COUNT];
        for (i, &mask) in input.iter().enumerate() {
            let (x, y) = from_index(i, width);
            for c in 0..T::COUNT as u8 {
                if mask & T::Bits::bit(c) != T::Bits::default() {
                    matches[c as usize].push((x, y));
                }
            }
        }
        Self {
            width,
            height,
            input,
            output,
            matches,
        }
    }

    pub fn matches_at(&self, grid: &GridData<T>, ox: u32, oy: u32) -> bool {
        if ox + self.width > grid.width || oy + self.height > grid.height {
            return false;
        }
        for (i, &bits) in self.input.iter().enumerate() {
            let (dx, dy) = from_index(i, self.width);
            let cell: u8 = grid[(ox + dx, oy + dy)].into();
            if bits & T::Bits::bit(cell) == T::Bits::default() {
                return false;
            }
        }
        true
    }
}

pub struct RuleSet<T: Cell> {
    rules: Vec<Rule<T>>,
}

impl<T: Cell> RuleSet<T> {
    pub fn new(
        width: u32,
        height: u32,
        input: Vec<T::Bits>,
        output: Vec<Option<T>>,
        symmetries: Symmetries,
    ) -> Self {
        assert_eq!(
            width as usize * height as usize,
            input.len(),
            "Invalid cell count for {width}x{height} grid: {}",
            input.len()
        );
        let mut rules = HashSet::new();
        for t in 1..8 {
            if symmetries.bits() & 1 << t > 0 {
                let (w, h, i) = Self::transform(width, height, &input, t);
                let (_, _, o) = Self::transform(width, height, &output, t);
                rules.insert((w, h, i, o));
            }
        }
        rules.insert((width, height, input, output));
        Self {
            rules: rules
                .into_iter()
                .map(|(w, h, i, o)| Rule::new(w, h, i, o))
                .collect(),
        }
    }

    pub fn union(&self, other: &Self) -> Self {
        let rules: HashSet<_> = self
            .iter()
            .chain(other.iter())
            .map(|r| (r.width, r.height, r.input.clone(), r.output.clone()))
            .collect();
        Self {
            rules: rules
                .into_iter()
                .map(|(w, h, i, o)| Rule::new(w, h, i, o))
                .collect(),
        }
    }

    fn iter(&self) -> std::slice::Iter<'_, Rule<T>> {
        self.rules.iter()
    }

    fn transform<U: Copy>(
        width: u32,
        height: u32,
        data: &[U],
        transform: u8,
    ) -> (u32, u32, Vec<U>) {
        let flip_x = transform & 0b100 != 0;
        let flip_y = transform & 0b010 != 0;
        let transpose = transform & 0b001 != 0;
        let mut result = Vec::with_capacity(data.len());
        let (out_width, out_height) = if transpose {
            (height, width)
        } else {
            (width, height)
        };
        for i in 0..data.len() {
            let (mut x, mut y) = from_index(i, out_width);
            if flip_x {
                x = out_width - 1 - x;
            }
            if flip_y {
                y = out_height - 1 - y;
            }
            if transpose {
                (x, y) = (y, x);
            }
            result.push(data[to_index(x, y, width)]);
        }
        (out_width, out_height, result)
    }
}

impl<T: Cell> Display for RuleSet<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, p) in self.rules.iter().enumerate() {
            writeln!(f, "{i}:")?;
            for (i, cell) in p.input.iter().enumerate() {
                if i != 0 && i % p.width as usize == 0 {
                    writeln!(f)?;
                }
                let value = cell;
                write!(
                    f,
                    " {value:0width$x}",
                    width = mem::size_of::<T::Bits>() * 2
                )?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl<T: Cell> BitOr<Self> for RuleSet<T> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(&rhs)
    }
}
