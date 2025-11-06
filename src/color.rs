//! B/W Color for EPDs
//!
//! EPD representation of multicolor with separate buffers
//! for each bit makes it hard to properly represent colors here

#[cfg(feature = "graphics")]
use embedded_graphics_core::pixelcolor::BinaryColor;
#[cfg(feature = "graphics")]
use embedded_graphics_core::pixelcolor::PixelColor;

/// When trying to parse u8 to one of the color types
#[derive(Debug, PartialEq, Eq)]
pub struct OutOfColorRangeParseError(u8);
impl core::fmt::Display for OutOfColorRangeParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Outside of possible Color Range: {}", self.0)
    }
}

impl OutOfColorRangeParseError {
    fn _new(size: u8) -> OutOfColorRangeParseError {
        OutOfColorRangeParseError(size)
    }
}

/// Only for the Black/White-Displays
// TODO : 'color' is not a good name for black and white, rename it to BiColor/BWColor ?
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Color {
    /// Black color
    Black,
    /// White color
    #[default]
    White,
}

/// Only for the Black/White/Color-Displays
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TriColor {
    /// Black color
    Black,
    /// White color
    #[default]
    White,
    /// Chromatic color
    Chromatic,
}

/// Only for the Black/White/Red/Yellow-Displays
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum QuadColor {
    /// Black color
    Black,
    /// White color
    #[default]
    White,
    /// Red color
    Red,
    /// Yellow color
    Yellow,
}

/// For the 7 Color Displays
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum OctColor {
    /// Black Color
    Black = 0x00,
    /// White Color
    #[default]
    White = 0x01,
    /// Green Color
    Green = 0x02,
    /// Blue Color
    Blue = 0x03,
    /// Red Color
    Red = 0x04,
    /// Yellow Color
    Yellow = 0x05,
    /// Orange Color
    Orange = 0x06,
    /// HiZ / Clean Color
    HiZ = 0x07,
}

/// Color trait for use in `Display`s
pub trait ColorType {
    /// Number of bit used to represent this color type in a single buffer.
    /// To get the real number of bits per pixel you should multiply this by `BUFFER_COUNT`
    const BITS_PER_PIXEL_PER_BUFFER: usize;

    /// Number of buffer used to represent this color type
    /// splitted buffer like tricolo is 2, otherwise this should be 1.
    const BUFFER_COUNT: usize;

    /// Return the data used to set a pixel color
    ///
    /// * bwrbit is used to tell the value of the unused bit when a chromatic
    ///   color is set (TriColor only as for now)
    /// * pos is the pixel position in the line, used to know which pixels must be set
    ///
    /// Return values are :
    /// * .0 is the mask used to exclude this pixel from the byte (eg: 0x7F in BiColor)
    /// * .1 are the bits used to set the color in the byte (eg: 0x80 in BiColor)
    ///      this is u16 because we set 2 bytes in case of split buffer
    fn bitmask(&self, bwrbit: bool, pos: u32) -> (u8, u16);

    /// convert 0 -> black, 1 -> white for binary color
    /// convert 00 -> black, 01 -> white, 11 -> red for tricolor
    /// and so on
    fn from_bits(bits: u8) -> Self;
}

impl ColorType for Color {
    const BITS_PER_PIXEL_PER_BUFFER: usize = 1;
    const BUFFER_COUNT: usize = 1;
    fn bitmask(&self, _bwrbit: bool, pos: u32) -> (u8, u16) {
        let bit = 0x80 >> (pos % 8);
        match self {
            Color::Black => (!bit, 0u16),
            Color::White => (!bit, bit as u16),
        }
    }

    fn from_bits(bits: u8) -> Self {
        match bits {
            0x00 => Color::Black,
            _ => Color::White,
        }
    }
}

impl ColorType for TriColor {
    const BITS_PER_PIXEL_PER_BUFFER: usize = 1;
    const BUFFER_COUNT: usize = 2;
    fn bitmask(&self, bwrbit: bool, pos: u32) -> (u8, u16) {
        let bit = 0x80 >> (pos % 8);
        match self {
            TriColor::Black => (!bit, 0u16),
            TriColor::White => (!bit, bit as u16),
            TriColor::Chromatic => (
                !bit,
                if bwrbit {
                    (bit as u16) << 8
                } else {
                    (bit as u16) << 8 | bit as u16
                },
            ),
        }
    }

    fn from_bits(bits: u8) -> Self {
        match bits {
            0x00 => TriColor::Black,
            0x01 => TriColor::Chromatic,
            _ => TriColor::White,
        }
    }
}

impl ColorType for QuadColor {
    // 每个缓冲区中每个像素占2位
    const BITS_PER_PIXEL_PER_BUFFER: usize = 2;
    // 使用1个缓冲区来存储2位颜色信息
    const BUFFER_COUNT: usize = 1;

    fn bitmask(&self, _bwrbit: bool, pos: u32) -> (u8, u16) {
        // 计算当前像素在字节中的起始位位置（每像素占2位）
        let shift = (pos % 4) * 2;
        // 掩码：清除当前像素的2位
        let mask = !(0x03 << shift);
        // 根据颜色获取对应的2位值
        let color_bits = match self {
            QuadColor::Black => 0b00,  // 黑色 0b00
            QuadColor::White => 0b01,  // 白色 0b01
            QuadColor::Yellow => 0b10, // 黄色 0b10
            QuadColor::Red => 0b11,    // 红色 0b11
        };
        // 将颜色值移到正确的位置
        let value = (color_bits << shift) as u16;

        (mask, value)
    }

    fn from_bits(bits: u8) -> Self {
        match bits {
            0x01 => QuadColor::White,
            0x02 => QuadColor::Yellow,
            0x11 => QuadColor::Red,
            _ => QuadColor::Black,
        }
    }
}

impl ColorType for OctColor {
    const BITS_PER_PIXEL_PER_BUFFER: usize = 4;
    const BUFFER_COUNT: usize = 1;
    fn bitmask(&self, _bwrbit: bool, pos: u32) -> (u8, u16) {
        let mask = !(0xF0 >> ((pos % 2) * 4));
        let bits = self.get_nibble() as u16;
        (mask, if pos % 2 == 1 { bits } else { bits << 4 })
    }

    fn from_bits(bits: u8) -> Self {
        match bits {
            0x00 => OctColor::Black,
            0x01 => OctColor::White,
            0x02 => OctColor::Yellow,
            // TODO:
            _ => OctColor::Red,
        }
    }
}

#[cfg(feature = "graphics")]
impl From<BinaryColor> for OctColor {
    fn from(b: BinaryColor) -> OctColor {
        match b {
            BinaryColor::On => OctColor::Black,
            BinaryColor::Off => OctColor::White,
        }
    }
}

#[cfg(feature = "graphics")]
impl From<OctColor> for embedded_graphics_core::pixelcolor::Rgb888 {
    fn from(b: OctColor) -> Self {
        let (r, g, b) = b.rgb();
        Self::new(r, g, b)
    }
}

#[cfg(feature = "graphics")]
impl From<embedded_graphics_core::pixelcolor::Rgb888> for OctColor {
    fn from(p: embedded_graphics_core::pixelcolor::Rgb888) -> OctColor {
        use embedded_graphics_core::prelude::RgbColor;
        let colors = [
            OctColor::Black,
            OctColor::White,
            OctColor::Green,
            OctColor::Blue,
            OctColor::Red,
            OctColor::Yellow,
            OctColor::Orange,
            OctColor::HiZ,
        ];
        // if the user has already mapped to the right color space, it will just be in the list
        if let Some(found) = colors.iter().find(|c| c.rgb() == (p.r(), p.g(), p.b())) {
            return *found;
        }

        // This is not ideal but just pick the nearest color
        *colors
            .iter()
            .map(|c| (c, c.rgb()))
            .map(|(c, (r, g, b))| {
                let dist = (i32::from(r) - i32::from(p.r())).pow(2)
                    + (i32::from(g) - i32::from(p.g())).pow(2)
                    + (i32::from(b) - i32::from(p.b())).pow(2);
                (c, dist)
            })
            .min_by_key(|(_c, dist)| *dist)
            .map(|(c, _)| c)
            .unwrap_or(&OctColor::White)
    }
}

#[cfg(feature = "graphics")]
impl From<embedded_graphics_core::pixelcolor::raw::RawU4> for OctColor {
    fn from(b: embedded_graphics_core::pixelcolor::raw::RawU4) -> Self {
        use embedded_graphics_core::prelude::RawData;
        OctColor::from_nibble(b.into_inner()).unwrap()
    }
}

#[cfg(feature = "graphics")]
impl PixelColor for OctColor {
    type Raw = embedded_graphics_core::pixelcolor::raw::RawU4;
}

impl OctColor {
    /// Gets the Nibble representation of the Color as needed by the display
    pub fn get_nibble(self) -> u8 {
        self as u8
    }
    /// Converts two colors into a single byte for the Display
    pub fn colors_byte(a: OctColor, b: OctColor) -> u8 {
        a.get_nibble() << 4 | b.get_nibble()
    }

    ///Take the nibble (lower 4 bits) and convert to an OctColor if possible
    pub fn from_nibble(nibble: u8) -> Result<OctColor, OutOfColorRangeParseError> {
        match nibble & 0xf {
            0x00 => Ok(OctColor::Black),
            0x01 => Ok(OctColor::White),
            0x02 => Ok(OctColor::Green),
            0x03 => Ok(OctColor::Blue),
            0x04 => Ok(OctColor::Red),
            0x05 => Ok(OctColor::Yellow),
            0x06 => Ok(OctColor::Orange),
            0x07 => Ok(OctColor::HiZ),
            e => Err(OutOfColorRangeParseError(e)),
        }
    }
    ///Split the nibbles of a single byte and convert both to an OctColor if possible
    pub fn split_byte(byte: u8) -> Result<(OctColor, OctColor), OutOfColorRangeParseError> {
        let low = OctColor::from_nibble(byte & 0xf)?;
        let high = OctColor::from_nibble((byte >> 4) & 0xf)?;
        Ok((high, low))
    }
    /// Converts to limited range of RGB values.
    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            OctColor::White => (0xff, 0xff, 0xff),
            OctColor::Black => (0x00, 0x00, 0x00),
            OctColor::Green => (0x00, 0xff, 0x00),
            OctColor::Blue => (0x00, 0x00, 0xff),
            OctColor::Red => (0xff, 0x00, 0x00),
            OctColor::Yellow => (0xff, 0xff, 0x00),
            OctColor::Orange => (0xff, 0x80, 0x00),
            OctColor::HiZ => (0x80, 0x80, 0x80), /* looks greyish */
        }
    }
}
//TODO: Rename get_bit_value to bit() and get_byte_value to byte() ?

impl Color {
    /// Get the color encoding of the color for one bit
    pub fn get_bit_value(self) -> u8 {
        match self {
            Color::White => 1u8,
            Color::Black => 0u8,
        }
    }

    /// Gets a full byte of black or white pixels
    pub fn get_byte_value(self) -> u8 {
        match self {
            Color::White => 0xff,
            Color::Black => 0x00,
        }
    }

    /// Parses from u8 to Color
    fn from_u8(val: u8) -> Self {
        match val {
            0 => Color::Black,
            1 => Color::White,
            e => panic!(
                "DisplayColor only parses 0 and 1 (Black and White) and not `{}`",
                e
            ),
        }
    }

    /// Returns the inverse of the given color.
    ///
    /// Black returns White and White returns Black
    pub fn inverse(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

impl From<u8> for Color {
    fn from(value: u8) -> Self {
        Color::from_u8(value)
    }
}

#[cfg(feature = "graphics")]
impl From<embedded_graphics_core::pixelcolor::raw::RawU1> for Color {
    fn from(b: embedded_graphics_core::pixelcolor::raw::RawU1) -> Self {
        use embedded_graphics_core::prelude::RawData;
        if b.into_inner() == 0 {
            Color::White
        } else {
            Color::Black
        }
    }
}

#[cfg(feature = "graphics")]
impl From<Color> for embedded_graphics_core::pixelcolor::raw::RawU1 {
    fn from(color: Color) -> Self {
        Self::new(color.get_bit_value())
    }
}

#[cfg(feature = "graphics")]
impl PixelColor for Color {
    type Raw = embedded_graphics_core::pixelcolor::raw::RawU1;
}

#[cfg(feature = "graphics")]
impl From<BinaryColor> for Color {
    fn from(b: BinaryColor) -> Color {
        match b {
            BinaryColor::On => Color::Black,
            BinaryColor::Off => Color::White,
        }
    }
}

#[cfg(feature = "graphics")]
impl From<embedded_graphics_core::pixelcolor::Rgb888> for Color {
    fn from(rgb: embedded_graphics_core::pixelcolor::Rgb888) -> Self {
        use embedded_graphics_core::pixelcolor::RgbColor;
        if rgb == RgbColor::BLACK {
            Color::Black
        } else if rgb == RgbColor::WHITE {
            Color::White
        } else {
            // choose closest color
            if (rgb.r() as u16 + rgb.g() as u16 + rgb.b() as u16) > 255 * 3 / 2 {
                Color::White
            } else {
                Color::Black
            }
        }
    }
}

#[cfg(feature = "graphics")]
impl From<Color> for embedded_graphics_core::pixelcolor::Rgb888 {
    fn from(color: Color) -> Self {
        use embedded_graphics_core::pixelcolor::RgbColor;
        match color {
            Color::Black => Self::BLACK,
            Color::White => Self::WHITE,
        }
    }
}

#[cfg(feature = "graphics")]
impl From<embedded_graphics_core::pixelcolor::Rgb565> for Color {
    fn from(rgb: embedded_graphics_core::pixelcolor::Rgb565) -> Self {
        use embedded_graphics_core::pixelcolor::RgbColor;
        if rgb == RgbColor::BLACK {
            Color::Black
        } else if rgb == RgbColor::WHITE {
            Color::White
        } else {
            // choose closest color
            if (rgb.r() as u16 + rgb.g() as u16 + rgb.b() as u16) > 255 * 3 / 2 {
                Color::White
            } else {
                Color::Black
            }
        }
    }
}

#[cfg(feature = "graphics")]
impl From<Color> for embedded_graphics_core::pixelcolor::Rgb565 {
    fn from(color: Color) -> Self {
        use embedded_graphics_core::pixelcolor::RgbColor;
        match color {
            Color::Black => Self::BLACK,
            Color::White => Self::WHITE,
        }
    }
}

#[cfg(feature = "graphics")]
impl From<embedded_graphics_core::pixelcolor::Rgb555> for Color {
    fn from(rgb: embedded_graphics_core::pixelcolor::Rgb555) -> Self {
        use embedded_graphics_core::pixelcolor::RgbColor;
        if rgb == RgbColor::BLACK {
            Color::Black
        } else if rgb == RgbColor::WHITE {
            Color::White
        } else {
            // choose closest color
            if (rgb.r() as u16 + rgb.g() as u16 + rgb.b() as u16) > 255 * 3 / 2 {
                Color::White
            } else {
                Color::Black
            }
        }
    }
}

#[cfg(feature = "graphics")]
impl From<Color> for embedded_graphics_core::pixelcolor::Rgb555 {
    fn from(color: Color) -> Self {
        use embedded_graphics_core::pixelcolor::RgbColor;
        match color {
            Color::Black => Self::BLACK,
            Color::White => Self::WHITE,
        }
    }
}

impl TriColor {
    /// Get the color encoding of the color for one bit
    pub fn get_bit_value(self) -> u8 {
        match self {
            TriColor::White => 1u8,
            TriColor::Black | TriColor::Chromatic => 0u8,
        }
    }

    /// Gets a full byte of black or white pixels
    pub fn get_byte_value(self) -> u8 {
        match self {
            TriColor::White => 0xff,
            TriColor::Black | TriColor::Chromatic => 0x00,
        }
    }
}

#[cfg(feature = "graphics")]
impl From<embedded_graphics_core::pixelcolor::raw::RawU2> for TriColor {
    fn from(b: embedded_graphics_core::pixelcolor::raw::RawU2) -> Self {
        use embedded_graphics_core::prelude::RawData;
        if b.into_inner() == 0b00 {
            TriColor::White
        } else if b.into_inner() == 0b01 {
            TriColor::Black
        } else {
            TriColor::Chromatic
        }
    }
}

#[cfg(feature = "graphics")]
impl PixelColor for TriColor {
    type Raw = embedded_graphics_core::pixelcolor::raw::RawU2;
}

#[cfg(feature = "graphics")]
impl From<BinaryColor> for TriColor {
    fn from(b: BinaryColor) -> TriColor {
        match b {
            BinaryColor::On => TriColor::Black,
            BinaryColor::Off => TriColor::White,
        }
    }
}
#[cfg(feature = "graphics")]
impl From<embedded_graphics_core::pixelcolor::Rgb888> for TriColor {
    fn from(rgb: embedded_graphics_core::pixelcolor::Rgb888) -> Self {
        use embedded_graphics_core::pixelcolor::RgbColor;
        if rgb == RgbColor::BLACK {
            TriColor::Black
        } else if rgb == RgbColor::WHITE {
            TriColor::White
        } else {
            // there is no good approximation here since we don't know which color is 'chromatic'
            TriColor::Chromatic
        }
    }
}
#[cfg(feature = "graphics")]
impl From<TriColor> for embedded_graphics_core::pixelcolor::Rgb888 {
    fn from(tri_color: TriColor) -> Self {
        use embedded_graphics_core::pixelcolor::RgbColor;
        match tri_color {
            TriColor::Black => embedded_graphics_core::pixelcolor::Rgb888::BLACK,
            TriColor::White => embedded_graphics_core::pixelcolor::Rgb888::WHITE,
            // assume chromatic is red
            TriColor::Chromatic => embedded_graphics_core::pixelcolor::Rgb888::new(255, 0, 0),
        }
    }
}

#[cfg(feature = "graphics")]
impl PixelColor for QuadColor {
    type Raw = embedded_graphics_core::pixelcolor::raw::RawU2;
}

impl QuadColor {
    /// Get the color encoding of the color for one bit
    pub fn get_bit_value(self) -> u8 {
        match self {
            QuadColor::White => 1u8,
            _ => 0u8,
        }
    }

    /// Gets a full byte of black or white pixels
    pub fn get_byte_value(self) -> u8 {
        match self {
            QuadColor::White => 0xff,
            _ => 0x00,
        }
    }
}

#[cfg(feature = "graphics")]
impl From<BinaryColor> for QuadColor {
    fn from(b: BinaryColor) -> QuadColor {
        match b {
            BinaryColor::On => QuadColor::Black,
            BinaryColor::Off => QuadColor::White,
        }
    }
}

#[cfg(feature = "graphics")]
impl From<embedded_graphics_core::pixelcolor::Rgb888> for QuadColor {
    fn from(rgb: embedded_graphics_core::pixelcolor::Rgb888) -> Self {
        use embedded_graphics_core::pixelcolor::RgbColor;
        if rgb == RgbColor::BLACK {
            QuadColor::Black
        } else if rgb == RgbColor::WHITE {
            QuadColor::White
        } else if rgb == RgbColor::YELLOW {
            QuadColor::Yellow
        } else {
            QuadColor::Red
        }
    }
}

#[cfg(feature = "graphics")]
impl From<QuadColor> for embedded_graphics_core::pixelcolor::Rgb888 {
    fn from(quad_color: QuadColor) -> Self {
        use embedded_graphics_core::pixelcolor::RgbColor;
        match quad_color {
            // 因电脑屏幕太亮，模拟器里面直接显示白色、黄色体验很差，稍微修改了颜色
            QuadColor::Black => embedded_graphics_core::pixelcolor::Rgb888::new(10, 10, 10),
            QuadColor::White => embedded_graphics_core::pixelcolor::Rgb888::new(240, 240, 240),
            QuadColor::Yellow => embedded_graphics_core::pixelcolor::Rgb888::new(240, 240, 100),
            QuadColor::Red => embedded_graphics_core::pixelcolor::Rgb888::new(200, 50, 50),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_u8() {
        assert_eq!(Color::Black, Color::from(0u8));
        assert_eq!(Color::White, Color::from(1u8));
    }

    // test all values aside from 0 and 1 which all should panic
    #[test]
    fn from_u8_panic() {
        for val in 2..=u8::MAX {
            extern crate std;
            let result = std::panic::catch_unwind(|| Color::from(val));
            assert!(result.is_err());
        }
    }

    #[test]
    fn u8_conversion_black() {
        assert_eq!(Color::from(Color::Black.get_bit_value()), Color::Black);
        assert_eq!(Color::from(0u8).get_bit_value(), 0u8);
    }

    #[test]
    fn u8_conversion_white() {
        assert_eq!(Color::from(Color::White.get_bit_value()), Color::White);
        assert_eq!(Color::from(1u8).get_bit_value(), 1u8);
    }

    #[test]
    fn test_oct() {
        let left = OctColor::Red;
        let right = OctColor::Green;
        assert_eq!(
            OctColor::split_byte(OctColor::colors_byte(left, right)),
            Ok((left, right))
        );
    }

    #[test]
    fn test_tricolor_bitmask() {
        assert_eq!(
            TriColor::Black.bitmask(false, 0),
            (0b01111111, u16::from_le_bytes([0b00000000, 0b00000000]))
        );
        assert_eq!(
            TriColor::White.bitmask(false, 0),
            (0b01111111, u16::from_le_bytes([0b10000000, 0b00000000]))
        );
        assert_eq!(
            TriColor::Chromatic.bitmask(false, 0),
            (0b01111111, u16::from_le_bytes([0b10000000, 0b10000000]))
        );

        assert_eq!(
            TriColor::Black.bitmask(true, 0),
            (0b01111111, u16::from_le_bytes([0b00000000, 0b00000000]))
        );
        assert_eq!(
            TriColor::White.bitmask(true, 0),
            (0b01111111, u16::from_le_bytes([0b10000000, 0b00000000]))
        );
        assert_eq!(
            TriColor::Chromatic.bitmask(true, 0),
            (0b01111111, u16::from_le_bytes([0b00000000, 0b10000000]))
        );
    }
}
