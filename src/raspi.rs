//! module for initialize raspi I2C
use rppal::i2c::{Error, I2c};

pub(crate) fn init_raspi() -> Result<I2c, Error> {
    let i2c = I2c::new()?;
    i2c.set_timeout(100)?;
    return Ok(i2c);
}
