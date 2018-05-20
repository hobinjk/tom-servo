extern crate i2cdev;
extern crate webthing;

#[macro_use]
extern crate serde_json;

use std::thread;
use std::time::Duration;

use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

use std::sync::{Arc, RwLock, Weak};

use webthing::{BaseProperty, BaseThing, Thing, Action, WebThingServer};
use webthing::server::ActionGenerator;
use webthing::property::ValueForwarder;

const SERVO_HAT_ADDR: u16 = 0x40;

struct Generator;

impl ActionGenerator for Generator {
    fn generate(
        &self,
        _thing: Weak<RwLock<Box<Thing>>>,
        _name: String,
        _input: Option<&serde_json::Value>,
    ) -> Option<Box<Action>> {
        None
    }
}

struct ServoValueForwarder {
    addr: u8,
    dev: Arc<RwLock<LinuxI2CDevice>>
}

impl ValueForwarder for ServoValueForwarder {
    fn set_value(&mut self, value: serde_json::Value) -> Result<serde_json::Value, &'static str> {
        println!("On-State is now {}", value);
        let min = 836.0;
        let full = 414.0 * 2.0;
        match value {
            serde_json::Value::Number(val) => {
                let bin_val = (val.as_f64().unwrap_or(0.0) / 100.0 * full + min) as u16;
                match self.dev.write().unwrap().smbus_write_word_data(self.addr, bin_val) {
                    Ok(_) => Ok(serde_json::Value::Number(val)),
                    Err(_) => Err("Unknown i2c error")
                }
            }
            v => {
                Ok(v)
            }
        }
    }
}

fn main() {
    let mut dev = LinuxI2CDevice::new("/dev/i2c-1", SERVO_HAT_ADDR).unwrap();

    dev.smbus_write_byte_data(0x00, 0x20).unwrap();
    dev.smbus_write_byte_data(0xfe, 0x1e).unwrap();

    dev.smbus_write_word_data(0x06, 0).unwrap();
    dev.smbus_write_word_data(0x08, 1250).unwrap();
    dev.smbus_write_word_data(0x0a, 0).unwrap();
    dev.smbus_write_word_data(0x0c, 1250).unwrap();

    let dev = Arc::new(RwLock::new(dev));

    thread::sleep(Duration::from_millis(100));

    let mut thing = BaseThing::new(
        "Camera Mount".to_owned(),
        Some("thing".to_owned()),
        None,
    );

    let servo0_description = json!({
        "type": "number",
        "description": "Servo 0 rotation"
    });
    let servo0_description = servo0_description.as_object().unwrap().clone();
    let servo1_description = json!({
        "type": "number",
        "description": "Servo 1 rotation"
    });
    let servo1_description = servo1_description.as_object().unwrap().clone();
    thing.add_property(Box::new(BaseProperty::new(
        "servo0".to_owned(),
        json!(50),
        Some(Box::new(ServoValueForwarder { dev: dev.clone(), addr: 0x08 })),
        Some(servo0_description),
    )));

    thing.add_property(Box::new(BaseProperty::new(
        "servo1".to_owned(),
        json!(50),
        Some(Box::new(ServoValueForwarder { dev: dev.clone(), addr: 0x0c })),
        Some(servo1_description),
    )));

    let mut things: Vec<Arc<RwLock<Box<Thing + 'static>>>> = Vec::new();
    things.push(Arc::new(RwLock::new(Box::new(thing))));
    let server = WebThingServer::new(things, None, Some(8888), None, Box::new(Generator));
    server.start();
}
