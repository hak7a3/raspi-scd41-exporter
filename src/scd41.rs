//! module for manipurate scd41
//! see https://sensirion.com/media/documents/48C4B7FB/66E05452/CD_DS_SCD4x_Datasheet_D1.pdf
use std::{thread, time::Duration};

use embedded_hal::i2c;
use sensirion_i2c::{crc8, i2c::{read_words_with_crc, write_command_u16, Error}};

const SCD41_I2C_ADDR: u8 = 0x62;

pub(crate) struct Measurement {
    pub(crate) co2: u16,
    pub(crate) temperature: f32,
    pub(crate) humidity: f32,
}

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
    let serial = ((buf[0] as u64) << 40)
        | ((buf[1] as u64) << 32)
        | ((buf[3] as u64) << 24)
        | ((buf[4] as u64) << 16)
        | ((buf[6] as u64) << 8)
        | (buf[7] as u64);
    return Ok(serial);
}

/// data ready (0xE4B8)
pub(crate) fn get_data_ready_status<I: i2c::I2c>(i2c: &mut I) -> Result<bool, Error<I>> {
    write_command_u16(i2c, SCD41_I2C_ADDR, 0xE4B8).map_err(Error::I2cWrite)?;
    thread::sleep(Duration::from_millis(1));

    let mut buf = [0; 3];
    read_words_with_crc(i2c, SCD41_I2C_ADDR, &mut buf)?;
    let status = ((buf[0] as u16) << 8) | (buf[1] as u16);
    log::info!("ready value {:x}", status);
    return Ok((status & 0x7FF) != 0);
}

/// read_measurement (0xEC05)
pub(crate) fn read_measurement<I: i2c::I2c>(i2c: &mut I) -> Result<Measurement, Error<I>> {
    write_command_u16(i2c, SCD41_I2C_ADDR, 0xEC05).map_err(Error::I2cWrite)?;
    thread::sleep(Duration::from_millis(1));

    let mut buf = [0; 9];
    read_words_with_crc(i2c, SCD41_I2C_ADDR, &mut buf)?;

    let raw_co2 = ((buf[0] as u16) << 8) | (buf[1] as u16);
    let raw_temperature = ((buf[3] as u16) << 8) | (buf[4] as u16);
    let raw_humidity = ((buf[6] as u16) << 8) | (buf[7] as u16);

    return Ok(Measurement {
        co2: raw_co2,
        temperature: raw_temperature as f32 * 175_f32 / 65535_f32 - 45_f32,
        humidity: raw_humidity as f32 * 100_f32 / 65535_f32,
    });
}

#[allow(dead_code)]
/// get_temperature_offset (0x2318)
pub(crate) fn get_temperature_offset<I: i2c::I2c>(i2c: &mut I) -> Result<f32, Error<I>> {
    write_command_u16(i2c, SCD41_I2C_ADDR, 0x2318).map_err(Error::I2cWrite)?;
    thread::sleep(Duration::from_millis(1));

    let mut buf = [0; 3];
    read_words_with_crc(i2c, SCD41_I2C_ADDR, &mut buf)?;
    let offset = ((buf[0] as u16) << 8) | (buf[1] as u16);
    return Ok(offset as f32 * 175_f32 / 65535_f32);
}

/// set_temperature_offset (0x241d)
pub(crate) fn set_temperature_offset<I: i2c::I2c>(i2c: &mut I, offset: f32) -> Result<(), Error<I>> {
    let offset = offset * 65535_f32 / 175_f32;
    let offset = offset as u16;
    let data = offset.to_be_bytes();

    let mut buf = [0_u8; 5];
    buf[0..2].copy_from_slice(&(0x241d_u16).to_be_bytes());
    buf[2..4].copy_from_slice(&data);
    buf[4] = crc8::calculate(&data);

    i2c.write(SCD41_I2C_ADDR, &buf).map_err(Error::I2cWrite)?;
    return Ok(());
}
