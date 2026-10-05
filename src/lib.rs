use std::{
    collections::HashSet,
    fmt::{Display, LowerHex},
    hash::Hash,
    mem,
    ops::{BitAnd, BitOr, Index, IndexMut, Not},
};

use bitflags::bitflags;
use image::RgbImage;
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
macro_rules! pattern_set {
    ($name:ident, $cell:ident, $symmetries:expr, [
        $([$($item:tt)+])+
    ]) => {
        let $name = {
            #[allow(clippy::unused_unit)]
            let void = [$([$($crate::pattern_set!(@void $item)),+]),+];
            let width = void[0].len() as u32;
            let height = void.len() as u32;
            $crate::PatternSet::<$cell>::new(width, height, vec![
                $($($crate::pattern_set!(@item $cell, $item)),*),*
            ], $symmetries)
        };
    };

    (@item $cell:ident, $item:ident) => {{
        use $crate::Bits;
        <$cell as $crate::Cell>::Bits::bit($cell::$item.into())
    }};

    (@item $cell:ident, _) => {{
        use $crate::Bits;
        <$cell as $crate::Cell>::Bits::ALL
    }};

    (@item $cell:ident, [$($item:ident)+]) => {{
        use $crate::Bits;
        $(<$cell as $crate::Cell>::Bits::bit($cell::$item.into()))|+
    }};

    (@void $($_:tt)*) => {{
        ()
    }};
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

pub trait Cell: Into<u8> + Into<RGB8> + Eq + Default + Copy {
    const COUNT: usize;
    type Bits: Bits;
}

pub struct Grid<T: Cell> {
    width: u32,
    height: u32,
    data: Vec<T>,
}

impl<T: Cell> Grid<T> {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![T::default(); width as usize * height as usize],
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        debug_assert!(
            x < self.width && y < self.height,
            "position out of range: ({x}, {y}) - grid size is ({}, {})",
            self.width,
            self.height
        );
        x as usize + y as usize * self.width as usize
    }

    pub fn get(&self, x: u32, y: u32) -> Option<&T> {
        self.data.get(self.index(x, y))
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

impl<T: Cell> Display for Grid<T> {
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

impl<T: Cell> Index<(u32, u32)> for Grid<T> {
    type Output = T;

    fn index(&self, index: (u32, u32)) -> &Self::Output {
        &self.data[self.index(index.0, index.1)]
    }
}

impl<T: Cell> IndexMut<(u32, u32)> for Grid<T> {
    fn index_mut(&mut self, index: (u32, u32)) -> &mut Self::Output {
        let index = self.index(index.0, index.1);
        &mut self.data[index]
    }
}

bitflags! {
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
    }
}

pub struct Pattern<T: Cell> {
    width: u32,
    height: u32,
    data: Vec<T::Bits>,
    /// `matches[i]`: every position within the pattern where cell `i` can be matched
    matches: Vec<Vec<(u32, u32)>>,
}

impl<T: Cell> Pattern<T> {
    pub fn new(width: u32, height: u32, data: Vec<T::Bits>) -> Self {
        let mut matches = vec![Vec::new(); T::COUNT];
        for (i, &mask) in data.iter().enumerate() {
            let (x, y) = (i as u32 % width, i as u32 / width);
            for c in 0..T::COUNT as u8 {
                if mask & T::Bits::bit(c) != T::Bits::default() {
                    matches[c as usize].push((x, y));
                }
            }
        }
        Self {
            width,
            height,
            data,
            matches,
        }
    }
}

pub struct PatternSet<T: Cell> {
    patterns: Vec<Pattern<T>>,
}

impl<T: Cell> PatternSet<T> {
    pub fn new(width: u32, height: u32, data: Vec<T::Bits>, symmetries: Symmetries) -> Self {
        assert_eq!(
            width as usize * height as usize,
            data.len(),
            "Invalid cell count for {width}x{height} grid: {}",
            data.len()
        );
        let mut patterns = HashSet::new();
        for i in 1..8 {
            if symmetries.bits() & 1 << i > 0 {
                patterns.insert(Self::apply_transform(width, height, &data, i));
            }
        }
        patterns.insert((width, height, data));
        Self {
            patterns: patterns
                .into_iter()
                .map(|(width, height, data)| Pattern::new(width, height, data))
                .collect(),
        }
    }

    fn apply_transform(
        width: u32,
        height: u32,
        data: &[T::Bits],
        transform: u8,
    ) -> (u32, u32, Vec<T::Bits>) {
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
            let (mut x, mut y) = (i as u32 % out_width, i as u32 / out_width);
            if flip_x {
                x = out_width - 1 - x;
            }
            if flip_y {
                y = out_height - 1 - y;
            }
            if transpose {
                (x, y) = (y, x);
            }
            result.push(data[x as usize + y as usize * width as usize]);
        }
        (out_width, out_height, result)
    }
}

impl<T: Cell> Display for PatternSet<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, p) in self.patterns.iter().enumerate() {
            writeln!(f, "{i}:")?;
            for (i, cell) in p.data.iter().enumerate() {
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
