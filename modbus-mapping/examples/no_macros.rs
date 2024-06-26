/// TCP Modbus client without use of macros
use modbus_mapping::core::{HoldingRegisterMap, InputRegisterMap};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio_modbus::{
    client::{tcp::connect_slave, Reader, Writer},
    slave::Slave,
};

#[derive(Debug, Clone, Default)]
pub struct BatteryInputRegisters {
    pub power: f32,
}

#[derive(Debug, Clone, Default)]
pub struct BatteryHoldingRegisters {
    pub setpoint: u16,
}

#[async_trait::async_trait]
impl InputRegisterMap for BatteryInputRegisters {
    async fn update_from_input_registers(
        &mut self,
        client: &mut dyn Reader,
    ) -> tokio_modbus::Result<()> {
        let _words = match client.read_input_registers(0, 2).await? {
            Ok(power) => power,
            Err(exc) => return Ok(Err(exc)),
        };
        // TODO: set

        Ok(Err(tokio_modbus::Exception::IllegalDataAddress))
    }
}

#[async_trait::async_trait]
impl HoldingRegisterMap for BatteryHoldingRegisters {
    async fn update_from_holding_registers(
        &mut self,
        client: &mut dyn Reader,
    ) -> tokio_modbus::Result<()> {
        let words = match client.read_input_registers(0, 1).await? {
            Ok(power) => power,
            Err(exc) => return Ok(Err(exc)),
        };
        self.setpoint = words[0];

        Ok(Err(tokio_modbus::Exception::IllegalDataAddress))
    }

    async fn write_to_registers(&self, client: &mut dyn Writer) -> tokio_modbus::Result<()> {
        match client.write_single_register(5, 0).await? {
            Ok(_) => Ok(Ok(())),
            Err(exc) => Ok(Err(exc)),
        }
    }
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
