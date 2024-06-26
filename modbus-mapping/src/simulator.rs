use std::{
    collections::HashMap,
    fmt::Debug,
    marker::PhantomData,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

use crate::codec::Word;
use futures::future;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_modbus::{server, Address, Exception, Quantity, Request, Response};

#[derive(Debug, Clone, Default)]
/// A raw Modbus input and holding registers representation
pub struct Registers(HashMap<Address, Word>);

impl Registers {
    /// Insert new consecutive registers with `words` values starting at `addr` address.
    pub fn insert(&mut self, addr: Address, words: Vec<Word>) {
        for (i, value) in words.into_iter().enumerate() {
            let reg_addr = addr + i as Address;
            self.0.insert(reg_addr, value);
        }
    }

    /// Helper method to shrink the container size.
    pub fn shrink(&mut self) {
        self.0.shrink_to_fit()
    }

    /// Read `cnt` consecutive registers starting at `addr`.
    pub fn read(&self, addr: Address, cnt: Quantity) -> Result<Vec<Word>, Exception> {
        let mut response_values = vec![0; cnt.into()];
        for i in 0..cnt {
            let reg_addr = addr + i;
            if let Some(r) = self.0.get(&reg_addr) {
                response_values[i as usize] = *r;
            } else {
                return Err(Exception::IllegalDataAddress);
            }
        }

        Ok(response_values)
    }

    /// Write `words` into existing consecutive registers starting at `addr`.
    pub fn write(&mut self, addr: Address, words: &[Word]) -> Result<(), Exception> {
        for (i, value) in words.iter().enumerate() {
            let reg_addr = addr + i as Address;
            if let Some(r) = self.0.get_mut(&reg_addr) {
                *r = *value;
            } else {
                return Err(Exception::IllegalDataAddress);
            }
        }

        Ok(())
    }
}

/// Trait with complementary functionality to [`crate::core::InputRegisterMap`]
/// to update [`Registers`] from the type instance for the simulation purposes.
pub trait InputRegisterModel {
    /// Create new input register map.
    fn new_registers(&self) -> Registers;
    fn update_registers(&self, registers: &mut Registers) -> Result<(), Exception>;
}

/// Trait with complementary functionality to [`crate::core::HoldingRegisterMap`]
/// to link [`Registers`] to the type instance for the simulation purposes.
pub trait HoldingRegisterModel {
    fn new_registers(&self) -> Registers;
    fn update_registers(&self, registers: &mut Registers) -> Result<(), Exception>;
    fn update_self(&mut self, registers: &Registers) -> Result<(), Exception>;
}

#[derive(Debug, Clone)]
/// Modbus data store to be used in IO operations for the simulation purposes.
pub struct DataStore<I, H> {
    pub input_registers: Registers,
    input_register_model: PhantomData<I>,
    pub holding_registers: Registers,
    holding_register_model: PhantomData<H>,
}

impl<I, H> Default for DataStore<I, H>
where
    I: Default + InputRegisterModel,
    H: Default + HoldingRegisterModel,
{
    fn default() -> Self {
        Self {
            input_registers: I::default().new_registers(),
            input_register_model: PhantomData,
            holding_registers: H::default().new_registers(),
            holding_register_model: PhantomData,
        }
    }
}

impl<I, H> DataStore<I, H>
where
    I: InputRegisterModel,
    H: HoldingRegisterModel,
{
    /// Method to be used to implement [tokio_modbus::server::Service](https://docs.rs/tokio-modbus/latest/tokio_modbus/server/trait.Service.html).
    pub fn service_call(
        &mut self,
        holding_register_model: &mut H,
        req: Request,
    ) -> future::Ready<Result<Response, Exception>> {
        match req {
            Request::ReadInputRegisters(addr, cnt) => match self.input_registers.read(addr, cnt) {
                Ok(values) => future::ready(Ok(Response::ReadInputRegisters(values))),
                Err(err) => future::ready(Err(err)),
            },
            Request::ReadHoldingRegisters(addr, cnt) => {
                match self.holding_registers.read(addr, cnt) {
                    Ok(values) => future::ready(Ok(Response::ReadHoldingRegisters(values))),
                    Err(err) => future::ready(Err(err)),
                }
            }
            Request::WriteMultipleRegisters(addr, values) => {
                match self.holding_registers.write(addr, &values) {
                    Ok(_) => match holding_register_model.update_self(&self.holding_registers) {
                        Ok(_) => future::ready(Ok(Response::WriteMultipleRegisters(
                            addr,
                            values.len() as u16,
                        ))),
                        Err(err) => future::ready(Err(err)),
                    },
                    Err(err) => future::ready(Err(err)),
                }
            }
            Request::WriteSingleRegister(addr, value) => {
                match self
                    .holding_registers
                    .write(addr, std::slice::from_ref(&value))
                {
                    Ok(_) => match holding_register_model.update_self(&self.holding_registers) {
                        Ok(_) => future::ready(Ok(Response::WriteSingleRegister(addr, value))),
                        Err(err) => future::ready(Err(err)),
                    },
                    Err(err) => future::ready(Err(err)),
                }
            }
            _ => future::ready(Err(Exception::IllegalFunction)),
        }
    }
}

/// Modbus device simulator trait.
/// The type should use [DataStore] structure and keep it in sync with its holding and input register fields.
pub trait Device {
    type InputRegisters: Default + InputRegisterModel;
    type HoldingRegisters: Default + HoldingRegisterModel;

    fn service_call(&mut self, req: Request) -> future::Ready<Result<Response, Exception>>;

    fn update_state(&mut self);
}

#[derive(Debug, Clone)]
/// Wrapper around [Device] needed because of [tokio_modbus::server::Service](https://docs.rs/tokio-modbus/latest/tokio_modbus/server/trait.Service.html).
pub struct Simulator<D: Device>(pub Arc<Mutex<D>>);

impl<D: Device> Simulator<D> {
    pub fn new(device: D) -> Self {
        Self(Arc::new(Mutex::new(device)))
    }
}

impl<D: Device> tokio_modbus::server::Service for Simulator<D> {
    type Request = Request<'static>;
    type Future = future::Ready<Result<Response, Exception>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let device = &mut self.0.lock().unwrap();
        device.service_call(req)
    }
}

/// Utility function to run TCP simulator forever.
pub async fn run_tcp_simulator<D: Device + Clone + Sync + Send + 'static>(
    socket_addr: SocketAddr,
    simulator: Simulator<D>,
    state_update_period: std::time::Duration,
) {
    let simulator_clone = simulator.clone();
    let server_task = tokio::spawn(async move {
        let _ = run_tcp_server_context(socket_addr, simulator_clone).await;
    });

    let state_update_task = spawn_state_update_task(simulator, state_update_period);

    let _ = state_update_task.await;
    let _ = server_task.await;
}

/// Utility function to spawn and run simulator RTU simulator forever.
pub async fn run_rtu_simulator<D: Device + Clone + Sync + Send + 'static>(
    path: &str,
    baud_rate: u32,
    simulator: Simulator<D>,
    state_update_period: std::time::Duration,
) {
    let builder = tokio_serial::new(path, baud_rate);
    let serial_stream = tokio_serial::SerialStream::open(&builder).unwrap();
    let server = server::rtu::Server::new(serial_stream);
    let service = simulator.clone();

    let server_task = tokio::spawn(async move {
        if let Err(err) = server.serve_forever(service).await {
            eprintln!("{err}");
        };
    });

    let state_update_task = spawn_state_update_task(simulator, state_update_period);

    let _ = state_update_task.await;
    let _ = server_task.await;
}

async fn run_tcp_server_context<D: Device + Clone + Sync + Send + 'static>(
    socket_addr: SocketAddr,
    simulator: Simulator<D>,
) {
    let listener = TcpListener::bind(socket_addr).await.unwrap();
    let server = server::tcp::Server::new(listener);
    let new_service = |_socket_addr| Ok(Some(simulator.clone()));
    let on_connected = |stream, socket_addr| async move {
        server::tcp::accept_tcp_connection(stream, socket_addr, new_service)
    };
    let on_process_error = |err| {
        eprintln!("{err}");
    };
    server.serve(&on_connected, on_process_error).await.unwrap();
}

fn spawn_state_update_task<D: Device + Clone + Sync + Send + 'static>(
    simulator: Simulator<D>,
    state_update_period: std::time::Duration,
) -> JoinHandle<()> {
    let interval = tokio::time::interval(state_update_period);
    let mut stream = IntervalStream::new(interval);

    tokio::spawn(async move {
        while let Some(_instant) = stream.next().await {
            simulator.0.lock().unwrap().update_state();
        }
    })
}
