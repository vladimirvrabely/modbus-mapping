use modbus_mapping::simulator::InputRegisterModel;
use modbus_mapping::{
    HoldingRegisterMap, HoldingRegisterModel, InputRegisterMap, InputRegisterModel,
};

#[derive(Debug, Clone, Default, InputRegisterMap, InputRegisterModel)]
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

#[derive(Debug, Clone, Default, HoldingRegisterMap, HoldingRegisterModel)]
pub struct BatteryHoldingRegisters {
    #[modbus(addr = 0, ty = "i32", ord = "be", x = 0.01, unit = "W")]
    pub setpoint: f32,
}

fn main() {
    let x = BatteryInputRegisters {
        power: 1_000.0,
        state_of_energy: 3_000.0,
        voltage: 220.0,
        grid_frequency: 50.0,
    };
    println!("{:?}", &x);

    let r = x.new_registers();
    println!("{:?}", r);
}
