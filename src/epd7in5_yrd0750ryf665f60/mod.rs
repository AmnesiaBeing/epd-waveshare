//! A simple Driver for the yrd0750ryf665f60 via SPI
//!
//! # References
//!
//! - [Website](http://www.yrdlcd.com/-53580.html)

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::SpiDevice,
};

use crate::color::QuadColor;
use crate::interface::DisplayInterface;
use crate::traits::{InternalWiAdditions, RefreshLut, WaveshareDisplay};

pub(crate) mod command;
use self::command::Command;
use crate::buffer_len;

use log::{debug, info};

/// Full size buffer for use with the 7in5b EPD (yrd0750ryf665f60)
#[cfg(feature = "graphics")]
pub type Display7in5 = crate::graphics::Display<
    WIDTH,
    HEIGHT,
    false,
    { buffer_len(WIDTH as usize, HEIGHT as usize * 2) },
    QuadColor,
>;

/// Width of the display
pub const WIDTH: u32 = 800;
/// Height of the display
pub const HEIGHT: u32 = 480;
/// Default Background Color
pub const DEFAULT_BACKGROUND_COLOR: QuadColor = QuadColor::White;

/// Number of bytes for b/w buffer and same for chromatic buffer bits
const NUM_DISPLAY_BITS: usize = WIDTH as usize / 4 * HEIGHT as usize;
const IS_BUSY_LOW: bool = true;
const SINGLE_BYTE_WRITE: bool = false;

/// Epd7in5 (yrd0750ryf665f60) driver
///
pub struct Epd7in5<SPI, BUSY, DC, RST, DELAY> {
    /// Connection Interface
    interface: DisplayInterface<SPI, BUSY, DC, RST, DELAY, SINGLE_BYTE_WRITE>,
    /// Background Color
    color: QuadColor,
}

impl<SPI, BUSY, DC, RST, DELAY> InternalWiAdditions<SPI, BUSY, DC, RST, DELAY>
    for Epd7in5<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    fn init(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        // Reset the device
        // 手册要求，RST先拉低10ms，再拉高10ms，然后等待屏幕空闲
        self.interface.reset(delay, 20_000, 20_000);
        self.wait_until_idle(spi, delay)?;

        // 示例代码中的神秘命令，需要先执行这条命令才能正确初始化
        self.cmd_with_data(spi, Command::MisteryCommand1, &[0x78])?;

        self.cmd_with_data(spi, Command::PanelSetting, &[0x2F, 0x29])?;

        self.cmd_with_data(spi, Command::VcomAndDataIntervalSetting, &[0x37])?;

        self.cmd_with_data(spi, Command::SpiFlashControl, &[0x00, 0x00, 0x00, 0x00])?;

        self.cmd_with_data(spi, Command::PowerSavingSetting, &[0x88])?;

        // 示例代码中的神秘命令，需要先执行这条命令才能正确初始化
        self.cmd_with_data(spi, Command::MisteryCommand2, &[0x01])?;

        self.cmd_with_data(spi, Command::PllControl, &[0x08])?;

        self.command(spi, Command::PowerOn)?;
        self.wait_until_idle(spi, delay)?;

        Ok(())
    }
}

impl<SPI, BUSY, DC, RST, DELAY> WaveshareDisplay<SPI, BUSY, DC, RST, DELAY>
    for Epd7in5<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    type DisplayColor = QuadColor;
    fn new(
        spi: &mut SPI,
        busy: BUSY,
        dc: DC,
        rst: RST,
        delay: &mut DELAY,
        delay_us: Option<u32>,
    ) -> Result<Self, SPI::Error> {
        let interface = DisplayInterface::new(busy, dc, rst, delay_us);
        let color = DEFAULT_BACKGROUND_COLOR;

        let mut epd = Epd7in5 { interface, color };

        epd.init(spi, delay)?;

        Ok(epd)
    }

    fn wake_up(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.init(spi, delay)
    }

    fn sleep(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.cmd_with_data(spi, Command::PowerOff, &[0x00])?;
        self.wait_until_idle(spi, delay)?;
        self.cmd_with_data(spi, Command::DeepSleep, &[0xA5])?;
        Ok(())
    }

    fn update_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.cmd_with_data(
            spi,
            Command::DataStartTransmission1,
            &buffer[..NUM_DISPLAY_BITS],
        )?;
        Ok(())
    }

    fn update_partial_frame(
        &mut self,
        _spi: &mut SPI,
        _delay: &mut DELAY,
        _buffer: &[u8],
        _x: u32,
        _y: u32,
        _width: u32,
        _height: u32,
    ) -> Result<(), SPI::Error> {
        unimplemented!()
    }

    fn display_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::DisplayRefresh, &[0x00])?;
        delay.delay_us(500);
        self.wait_until_idle(spi, delay)?;
        Ok(())
    }

    fn update_and_display_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        info!("Updating and displaying frame on EPD7in5");
        self.update_frame(spi, buffer, delay)?;
        info!("Frame updated, now displaying");
        self.command(spi, Command::PowerOn)?;
        self.display_frame(spi, delay)?;
        Ok(())
    }

    fn clear_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.send_resolution(spi)?;

        self.command(spi, Command::DataStartTransmission1)?;
        // 白色是0b01，重复 Width*Height/4 次
        self.interface.data_x_times(spi, 0x55, WIDTH * HEIGHT / 4)?;

        self.interface.cmd(spi, Command::DataStop)?;

        self.command(spi, Command::DisplayRefresh)?;

        Ok(())
    }

    fn set_background_color(&mut self, color: Self::DisplayColor) {
        self.color = color;
    }

    fn background_color(&self) -> &Self::DisplayColor {
        &self.color
    }

    fn width(&self) -> u32 {
        WIDTH
    }

    fn height(&self) -> u32 {
        HEIGHT
    }

    fn set_lut(
        &mut self,
        _spi: &mut SPI,
        _delay: &mut DELAY,
        _refresh_rate: Option<RefreshLut>,
    ) -> Result<(), SPI::Error> {
        unimplemented!();
    }

    /// wait
    fn wait_until_idle(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        let _ = spi;
        self.interface.wait_until_idle(delay, IS_BUSY_LOW);
        Ok(())
    }
}

impl<SPI, BUSY, DC, RST, DELAY> Epd7in5<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    /// temporary replacement for missing delay in the trait to call wait_until_idle
    #[allow(clippy::too_many_arguments)]
    pub fn update_partial_frame2(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        if buffer.len() as u32 != width / 8 * height {
            //TODO panic or error
        }

        let hrst_upper = (x / 8) as u8 >> 5;
        let hrst_lower = ((x / 8) << 3) as u8;
        let hred_upper = ((x + width) / 8 - 1) as u8 >> 5;
        let hred_lower = (((x + width) / 8 - 1) << 3) as u8 | 0b111;
        let vrst_upper = (y >> 8) as u8;
        let vrst_lower = y as u8;
        let vred_upper = ((y + height - 1) >> 8) as u8;
        let vred_lower = (y + height - 1) as u8;
        let pt_scan = 0x01; // Gates scan both inside and outside of the partial window. (default)

        self.cmd_with_data(
            spi,
            Command::PartialWindow,
            &[
                hrst_upper, hrst_lower, hred_upper, hred_lower, vrst_upper, vrst_lower, vred_upper,
                vred_lower, pt_scan,
            ],
        )?;
        let half = buffer.len() / 2;
        self.cmd_with_data(spi, Command::DataStartTransmission1, &buffer[..half])?;

        self.command(spi, Command::DisplayRefresh)?;
        self.wait_until_idle(spi, delay)?;

        Ok(())
    }

    fn command(&mut self, spi: &mut SPI, command: Command) -> Result<(), SPI::Error> {
        self.interface.cmd(spi, command)
    }

    fn send_data(&mut self, spi: &mut SPI, data: &[u8]) -> Result<(), SPI::Error> {
        self.interface.data(spi, data)
    }

    fn cmd_with_data(
        &mut self,
        spi: &mut SPI,
        command: Command,
        data: &[u8],
    ) -> Result<(), SPI::Error> {
        self.interface.cmd_with_data(spi, command, data)
    }

    fn send_resolution(&mut self, spi: &mut SPI) -> Result<(), SPI::Error> {
        let w = self.width();
        let h = self.height();

        self.command(spi, Command::TconResolution)?;
        self.send_data(spi, &[(w >> 8) as u8])?;
        self.send_data(spi, &[w as u8])?;
        self.send_data(spi, &[(h >> 8) as u8])?;
        self.send_data(spi, &[h as u8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epd_size() {
        assert_eq!(WIDTH, 800);
        assert_eq!(HEIGHT, 480);
        assert_eq!(DEFAULT_BACKGROUND_COLOR, QuadColor::White);
    }
}
