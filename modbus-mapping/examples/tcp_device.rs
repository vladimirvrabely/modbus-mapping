/// Battery TCP Modbus simulator
use futures::future;
use modbus_mapping::simulator::{
    run_tcp_simulator, DataStore, Device, InputRegisterModel, Simulator,
};
use modbus_mapping::{HoldingRegisterModel, InputRegisterModel};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio_modbus::{Request, Response};

#[derive(Debug, Clone, Default, InputRegisterModel)]
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

#[derive(Debug, Clone, Default, HoldingRegisterModel)]
pub struct BatteryHoldingRegisters {
    #[modbus(addr = 0, ty = "i32", ord = "be", x = 0.01, unit = "W")]
    pub setpoint: f32,
}

#[derive(Debug, Clone, Default)]
struct Battery {
    ir: BatteryInputRegisters,
    hr: BatteryHoldingRegisters,
    data_store: DataStore<BatteryInputRegisters, BatteryHoldingRegisters>,
}

impl Device for Battery {
    type InputRegisters = BatteryInputRegisters;
    type HoldingRegisters = BatteryHoldingRegisters;

    fn service_call(&mut self, req: Request) -> future::Ready<Result<Response, std::io::Error>> {
        self.data_store.service_call(&mut self.hr, req)
    }

    fn update_state(&mut self) {
        eprintln!("Updating state");
        self.ir.power += 1.0;

        let _ = self
            .ir
            .update_registers(&mut self.data_store.input_registers);
    }
}

#[tokio::main]
async fn main() {
    let device = Battery::default();
    let simulator = Simulator::new(device);
    let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let state_update_period = std::time::Duration::from_millis(200);

    run_tcp_simulator(socket_addr, simulator, state_update_period).await;
}
