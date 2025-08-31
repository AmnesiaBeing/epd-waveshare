//! A simulator for Waveshare E-Ink Displays using embedded-graphics-simulator
//!
//! This simulator supports custom resolutions and color types.

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::SpiDevice,
};

use embedded_graphics_core::{
    pixelcolor::{PixelColor, Rgb888},
    prelude::*,
    primitives::Rectangle,
};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};

use crate::color::{ColorType, QuadColor};
use crate::interface::DisplayInterface;
use crate::traits::{InternalWiAdditions, RefreshLut, WaveshareDisplay};

/// EpdSimulator driver
///
/// Generic over width, height, and color type
pub struct EpdSimulator<COLOR, _SPI, _BUSY, _DC, _RST, _DELAY>
where
    COLOR: PixelColor + ColorType + Into<Rgb888> + From<Rgb888>,
{
    /// Screen width
    width: u32,
    /// Screen height
    height: u32,
    /// Background Color
    background_color: COLOR,
    /// Simulator window
    simulator_window: Option<core::cell::RefCell<Window>>,
    /// Simulator display
    simulator_display: SimulatorDisplay<COLOR>,
    /// Output settings for the simulator
    output_settings: embedded_graphics_simulator::OutputSettings,
    /// Dummy interface (not used in simulator)
    interface: DisplayInterface<_SPI, _BUSY, _DC, _RST, _DELAY, false>,
    /// Buffer for frame data
    buffer: Vec<u8>,
}

impl<COLOR, SPI, BUSY, DC, RST, DELAY> InternalWiAdditions<SPI, BUSY, DC, RST, DELAY>
    for EpdSimulator<COLOR, SPI, BUSY, DC, RST, DELAY>
where
    COLOR: PixelColor + ColorType + Default + Into<Rgb888> + From<Rgb888>,
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    fn init(&mut self, _spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        if self.simulator_window.is_none() {
            // Create and show the simulator window
            self.simulator_window = Some(core::cell::RefCell::new(Window::new(
                &format!("EPD Simulator {}x{}", self.width, self.height),
                &self.output_settings,
            )));
        }
        Ok(())
    }
}

impl<COLOR, SPI, BUSY, DC, RST, DELAY> WaveshareDisplay<SPI, BUSY, DC, RST, DELAY>
    for EpdSimulator<COLOR, SPI, BUSY, DC, RST, DELAY>
where
    COLOR: PixelColor + ColorType + Default + Clone + Into<Rgb888> + From<Rgb888>,
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    type DisplayColor = COLOR;

    // 保留 trait 要求的 new 方法（使用默认分辨率）
    fn new(
        spi: &mut SPI,
        busy: BUSY,
        dc: DC,
        rst: RST,
        delay: &mut DELAY,
        delay_us: Option<u32>,
    ) -> Result<Self, SPI::Error> {
        // 调用自定义构造函数，使用默认分辨率
        Self::new_with_size(spi, busy, dc, rst, delay, delay_us, 640, 384)
    }

    fn sleep(&mut self, _spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        // No action for simulator
        Ok(())
    }

    fn wake_up(&mut self, _spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        // No action for simulator
        Ok(())
    }

    fn set_background_color(&mut self, color: COLOR) {
        self.background_color = color;
    }

    fn background_color(&self) -> &COLOR {
        &self.background_color
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn update_frame(
        &mut self,
        _spi: &mut SPI,
        buffer: &[u8],
        _delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        // Update internal buffer
        self.buffer.copy_from_slice(buffer);

        // Convert buffer to pixels and update simulator display
        self.update_simulator_display();
        Ok(())
    }

    fn update_partial_frame(
        &mut self,
        _spi: &mut SPI,
        _delay: &mut DELAY,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), SPI::Error> {
        // Calculate the region to update
        let region = Rectangle::new(Point::new(x as i32, y as i32), Size::new(width, height));

        // Update the relevant part of the internal buffer
        let start_idx = ((y * self.width + x) / 8) as usize;
        let end_idx = start_idx + ((width * height) / 8) as usize;
        self.buffer[start_idx..end_idx].copy_from_slice(buffer);

        // Update only the relevant part of the simulator display
        self.update_simulator_region(region);
        Ok(())
    }

    fn display_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        // Refresh the simulator window
        // Window::new(
        //     &format!("EPD Simulator {}x{}", self.width, self.height),
        //     &self.output_settings,
        // )
        // .show_static(&self.simulator_display);
        let _ = self.init(spi, delay);
        if let Some(window) = &self.simulator_window {
            window.borrow_mut().show_static(&self.simulator_display);
        }
        Ok(())
    }

    fn update_and_display_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.update_frame(spi, buffer, delay)?;
        self.display_frame(spi, delay)
    }

    fn clear_frame(&mut self, _spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        // Clear internal buffer
        // self.buffer.fill(self.background_color.to_bits());
        self.buffer.fill(0);

        // 修正：Infallible 错误处理（clear 永远不会失败）
        self.simulator_display
            .clear(self.background_color.clone())
            .unwrap();

        Ok(())
    }

    fn set_lut(
        &mut self,
        _spi: &mut SPI,
        _delay: &mut DELAY,
        _refresh_rate: Option<RefreshLut>,
    ) -> Result<(), SPI::Error> {
        // Not implemented for simulator
        Ok(())
    }

    fn wait_until_idle(&mut self, _spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        // Simulator is always idle
        Ok(())
    }
}

// 问题 1 修正：将 new_with_size 作为结构体的关联函数（而非 trait 方法）
impl<COLOR, SPI, BUSY, DC, RST, DELAY> EpdSimulator<COLOR, SPI, BUSY, DC, RST, DELAY>
where
    COLOR: PixelColor + ColorType + Default + Clone + Into<Rgb888> + From<Rgb888>,
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    /// 创建自定义分辨率的模拟器（关联函数，而非 trait 方法）
    pub fn new_with_size(
        spi: &mut SPI,
        busy: BUSY,
        dc: DC,
        rst: RST,
        delay: &mut DELAY,
        delay_us: Option<u32>,
        width: u32,
        height: u32,
    ) -> Result<Self, SPI::Error> {
        let interface = DisplayInterface::new(busy, dc, rst, delay_us);

        let buffer_size = (width * height) as usize / 8 * COLOR::BITS_PER_PIXEL_PER_BUFFER;
        let buffer = vec![0u8; buffer_size];

        let output_settings = OutputSettingsBuilder::new().scale(1).build();

        let simulator_display =
            SimulatorDisplay::with_default_color(Size::new(width, height), COLOR::default());

        let mut epd = EpdSimulator {
            width,
            height,
            background_color: COLOR::default(),
            simulator_window: None,
            simulator_display,
            output_settings,
            interface,
            buffer,
        };

        epd.init(spi, delay)?;

        Ok(epd)
    }

    /// Update the entire simulator display from the internal buffer
    fn update_simulator_display(&mut self) {
        let region = Rectangle::new(Point::new(0, 0), Size::new(self.width, self.height));
        self.update_simulator_region(region);
    }

    /// Update a specific region of the simulator display from the internal buffer
    fn update_simulator_region(&mut self, region: Rectangle) {
        let start_x = region.top_left.x as u32;
        let start_y = region.top_left.y as u32;
        let width = region.size.width;
        let height = region.size.height;

        // Iterate over each pixel in the region
        for y in 0..height {
            for x in 0..width {
                let abs_x = start_x + x;
                let abs_y = start_y + y;

                // Calculate position in buffer
                let pos = abs_y * self.width + abs_x;
                let (mask, _) = COLOR::bitmask(&self.background_color, false, pos);

                let color = COLOR::from_bits(
                    self.buffer[pos as usize * COLOR::BITS_PER_PIXEL_PER_BUFFER / 8] & mask,
                );

                // Draw pixel to simulator display
                let _ =
                    embedded_graphics_core::Pixel(Point::new(abs_x as i32, abs_y as i32), color)
                        .draw(&mut self.simulator_display);
            }
        }
    }

    /// Set the simulator window scale for better visibility
    pub fn set_scale(&mut self, scale: u32) {
        self.output_settings = OutputSettingsBuilder::new().scale(scale).build();
    }
}
