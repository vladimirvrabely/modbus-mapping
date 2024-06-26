/// RTU Modbus client
use modbus_mapping::{core::InputRegisterMap, HoldingRegisterMap, InputRegisterMap};
use tokio_modbus::{client::rtu::attach_slave, slave::Slave};

#[derive(Debug, Clone, Default, InputRegisterMap)]
pub struct BatteryInputRegisters {
    #[modbus(addr = 0, ty = "u32", ord = "be", x = 1.0, unit = "W")]
    pub power: f32,
    #[modbus(addr = 2, ty = "u32", ord = "be", x = 100.0, unit = "Wh")]
    pub state_of_energy: f32,
    #[modbus(addr = 4, ty = "u32", ord = "be", x = 0.01, unit = "V")]
    pub voltage: f32,
    #[modbus(addr = 6, ty = "u32", ord = "be", x = 0.01, unit = "Hz")]
    pub grid_frequency: f32,
}

#[derive(Debug, Clone, Default, HoldingRegisterMap)]
pub struct BatteryHoldingRegisters {
    #[modbus(addr = 0, ty = "i32", ord = "be", x = 0.01, unit = "W")]
    pub setpoint: f32,
}

#[tokio::main]
async fn main() {
    let path = "/tmp/ttys002";
    let baud_rate = 0;
    let builder = tokio_serial::new(path, baud_rate);
    let serial_stream = tokio_serial::SerialStream::open(&builder).unwrap();
    let slave = Slave(0);
    let mut client = attach_slave(serial_stream, slave);

    loop {
        let ir = BatteryInputRegisters::from_input_registers(&mut client)
            .await
            .unwrap()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        println!("{:?}", ir);
    }
}
