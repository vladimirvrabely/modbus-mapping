use std::{
    collections::HashMap,
    fmt::Debug,
    marker::PhantomData,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use crate::codec::Word;
use futures::future;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_modbus::{server, Request, Response};
use tokio_modbus::{Address, Quantity};

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
    pub fn read(&self, addr: Address, cnt: Quantity) -> Result<Vec<Word>, std::io::Error> {
        let mut response_values = vec![0; cnt.into()];
        for i in 0..cnt {
            let reg_addr = addr + i;
            if let Some(r) = self.0.get(&reg_addr) {
                response_values[i as usize] = *r;
            } else {
                // TODO: Return a Modbus Exception response `IllegalDataAddress` https://github.com/slowtec/tokio-modbus/issues/165
                println!("SERVER: Exception::IllegalDataAddress");
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AddrNotAvailable,
                    format!("no register at address {reg_addr}"),
                ));
            }
        }

        Ok(response_values)
    }

    /// Write `words` into existing consecutive registers starting at `addr`.
    pub fn write(&mut self, addr: Address, words: &[Word]) -> Result<(), std::io::Error> {
        for (i, value) in words.iter().enumerate() {
            let reg_addr = addr + i as Address;
            if let Some(r) = self.0.get_mut(&reg_addr) {
                *r = *value;
            } else {
                // TODO: Return a Modbus Exception response `IllegalDataAddress` https://github.com/slowtec/tokio-modbus/issues/165
                println!("SERVER: Exception::IllegalDataAddress");
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AddrNotAvailable,
                    format!("no register at address {reg_addr}"),
                ));
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
    fn update_registers(&self, registers: &mut Registers) -> Result<(), std::io::Error>;
}

/// Trait with complementary functionality to [`crate::core::HoldingRegisterMap`]
/// to link [`Registers`] to the type instance for the simulation purposes.
pub trait HoldingRegisterModel {
    fn new_registers(&self) -> Registers;
    fn update_registers(&self, registers: &mut Registers) -> Result<(), std::io::Error>;
    fn update_self(&mut self, registers: &Registers) -> Result<(), std::io::Error>;
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
    ) -> future::Ready<Result<Response, std::io::Error>> {
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
            _ => {
                println!("SERVER: Exception::IllegalFunction - Unimplemented function code in request: {req:?}");
                // TODO: We want to return a Modbus Exception response `IllegalFunction`. https://github.com/slowtec/tokio-modbus/issues/165
                future::ready(Err(std::io::Error::new(
                    std::io::ErrorKind::AddrNotAvailable,
                    "Unimplemented function code in request".to_string(),
                )))
            }
        }
    }
}

/// Modbus device simulator trait.
/// The type should use [DataStore] structure and keep it in sync with its holding and input register fields.
pub trait Device {
    type InputRegisters: Default + InputRegisterModel;
    type HoldingRegisters: Default + HoldingRegisterModel;
    type Input;

    fn service_call(&mut self, req: Request) -> future::Ready<Result<Response, std::io::Error>>;

    fn update_state(&mut self, input: Self::Input);
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
    type Response = Response;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let device = &mut self.0.lock().unwrap();
        device.service_call(req)
    }
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

/// Utility function to spawn and run simulator TCP simulator forever.
pub fn spawn_tcp_simulator<D: Device + Clone + Sync + Send + 'static>(
    socket_addr: SocketAddr,
    simulator: Simulator<D>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let _ = run_tcp_server_context(socket_addr, simulator).await;
    })
}

/// Utility function to spawn and run simulator RTU simulator forever.
pub fn spawn_rtu_simulator<D: Device + Clone + Sync + Send + 'static>(
    path: &str,
    baud_rate: u32,
    simulator: Simulator<D>,
) -> JoinHandle<()> {
    let builder = tokio_serial::new(path, baud_rate);
    let serial_stream = tokio_serial::SerialStream::open(&builder).unwrap();
    let server = server::rtu::Server::new(serial_stream);
    let service = simulator.clone();

    tokio::spawn(async move {
        if let Err(err) = server.serve_forever(service).await {
            eprintln!("{err}");
        };
    })
}
