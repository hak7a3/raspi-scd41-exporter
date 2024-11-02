//! module for manipurate scd41
//! see https://sensirion.com/media/documents/48C4B7FB/66E05452/CD_DS_SCD4x_Datasheet_D1.pdf
use std::{thread, time::Duration};

use embedded_hal::i2c;
use sensirion_i2c::i2c::{read_words_with_crc, write_command_u16, Error};

const SCD41_I2C_ADDR: u8 = 0x62;

/// clean scd41's state.
pub(crate) fn clean_state<I: i2c::I2c>(i2c: &mut I) {
    let _ = wakeup(i2c).inspect_err(|e| log::trace!("wakeup error {:?}", e));
    let _ = stop_periodic_measurement(i2c).inspect_err(|e| log::trace!("stop error {:?}", e));
    let _ = reinit(i2c).inspect_err(|e| log::trace!("reinit error {:?}", e));
}

/// wakeup (0x36F6)
pub(crate) fn wakeup<I: i2c::I2c>(i2c: &mut I) -> Result<(), I::Error> {
    write_command_u16(i2c, SCD41_I2C_ADDR, 0x36F6)?;
    thread::sleep(Duration::from_millis(30));
    return Ok(());
}

/// start_periodic_measurement (0x21B1)
pub(crate) fn start_periodic_measurement<I: i2c::I2c>(i2c: &mut I) -> Result<(), I::Error> {
    write_command_u16(i2c, SCD41_I2C_ADDR, 0x21B1)?;
    thread::sleep(Duration::from_millis(1));
    return Ok(());
}

/// stop_periodic_measurement (0x3F86)
pub(crate) fn stop_periodic_measurement<I: i2c::I2c>(i2c: &mut I) -> Result<(), I::Error> {
    write_command_u16(i2c, SCD41_I2C_ADDR, 0x3F86)?;
    thread::sleep(Duration::from_millis(500));
    return Ok(());
}

/// reinit (0x3646)
pub(crate) fn reinit<I: i2c::I2c>(i2c: &mut I) -> Result<(), I::Error> {
    write_command_u16(i2c, SCD41_I2C_ADDR, 0x3646)?;
    thread::sleep(Duration::from_millis(30));
    return Ok(());
}

/// read_serial (0x3682)
pub(crate) fn read_serial<I: i2c::I2c>(i2c: &mut I) -> Result<u64, Error<I>> {
    write_command_u16(i2c, SCD41_I2C_ADDR, 0x3682).map_err(Error::I2cWrite)?;
    thread::sleep(Duration::from_millis(1));

    let mut buf = [0; 9];
    read_words_with_crc(i2c, SCD41_I2C_ADDR, &mut buf)?;
    return Ok(u64::from_be_bytes([
        0, 0, buf[0], buf[1], buf[3], buf[4], buf[6], buf[7],
    ]));
}

/// data ready (0xE4B8)
pub(crate) fn get_data_ready_status<I: i2c::I2c>(i2c: &mut I) -> Result<bool, Error<I>> {
    write_command_u16(i2c, SCD41_I2C_ADDR, 0xE4B8).map_err(Error::I2cWrite)?;
    thread::sleep(Duration::from_millis(1));

    let mut buf = [0; 3];
    read_words_with_crc(i2c, SCD41_I2C_ADDR, &mut buf)?;
    // TODO: crc check
    let status = u16::from_be_bytes([buf[0], buf[1]]);
    log::info!("ready value {:x}", status);
    return Ok((status & 0x7FF) != 0);
}

pub(crate) struct Measurement {
    pub(crate) co2: u16,
    pub(crate) temperature: f32,
    pub(crate) humidity: f32,
}

/// read_measurement (0xEC05)
pub(crate) fn read_measurement<I: i2c::I2c>(i2c: &mut I) -> Result<Measurement, Error<I>> {
    write_command_u16(i2c, SCD41_I2C_ADDR, 0xEC05).map_err(Error::I2cWrite)?;
    thread::sleep(Duration::from_millis(1));

    let mut buf = [0; 9];
    read_words_with_crc(i2c, SCD41_I2C_ADDR, &mut buf)?;

    let raw_co2 = u16::from_be_bytes([buf[0], buf[1]]);
    let raw_temperature = u16::from_be_bytes([buf[3], buf[4]]);
    let raw_humidity = u16::from_be_bytes([buf[6], buf[7]]);

    return Ok(Measurement {
        co2: raw_co2,
        temperature: raw_temperature as f32 * 175_f32 / 65536_f32 - 45_f32,
        humidity: raw_humidity as f32 * 100_f32 / 65536_f32,
    });
}
