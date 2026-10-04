use std::{
    fmt::Display, hash::Hash, ops::{BitAnd, BitOr, Index, IndexMut, Not},
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
        #[derive(Clone, Copy, Hash)]
        #[repr(u8)]
        $vis enum $name {
            $($cell,)*
        }

        impl $name {
            pub const VALUES: &[Self] = &[$($name::$cell),+];
            pub const COLORS: &[$crate::rgb::RGB8] = &[$($color),+];
            pub const COUNT: usize = Self::VALUES.len();
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
    Default + Copy + Eq + BitAnd<Output = Self> + BitOr<Output = Self> + Not<Output = Self>
{
    const ALL: Self;
    fn bit(index: u8) -> Self;
}

impl_bits!(u8, u16, u32, u64, u128);

pub trait Cell: Into<u8> + Into<RGB8> + Default + Copy + Hash {
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
        debug_assert!(x < self.width && y < self.height, "position out of range: ({x}, {y}) - grid size is ({}, {})", self.width, self.height);
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
    #[derive(Hash)]
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

#[derive(Hash)]
pub struct Pattern<T: Cell> {
    width: u32,
    height: u32,
    data: Vec<T::Bits>,
    symmetries: Symmetries,
}

impl<T: Cell> Pattern<T> {
    pub fn new(width: u32, height: u32, data: Vec<T::Bits>, symmetries: Symmetries) -> Self {
        Self {
            width,
            height,
            data,
            symmetries: symmetries | Symmetries::IDENTITY,
        }
    }
}
