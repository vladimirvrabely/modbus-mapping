/// TCP Modbus client
use modbus_mapping::{
    core::InputRegisterMap,
    derive::{HoldingRegisterMap, InputRegisterMap},
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio_modbus::{client::tcp::connect_slave, slave::Slave};

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
    let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let slave = Slave(0);

    let mut client = connect_slave(socket_addr, slave).await.unwrap();

    loop {
        let ir = BatteryInputRegisters::from_input_registers(&mut client)
            .await
            .unwrap()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        println!("{:?}", ir);
    }
}
