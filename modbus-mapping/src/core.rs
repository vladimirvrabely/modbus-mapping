use async_trait::async_trait;
use tokio_modbus::client::{Reader, Writer};

#[async_trait]
/// Define mapping between Modbus input registers and the Self type
/// to create and update the type instance by reading values directly from input registers
pub trait InputRegisterMap
where
    Self: Sized + Default,
{
    async fn update_from_input_registers(
        &mut self,
        client: &mut dyn Reader,
    ) -> Result<(), std::io::Error>;

    async fn from_input_registers(client: &mut dyn Reader) -> Result<Self, std::io::Error> {
        let mut new = Self::default();
        new.update_from_input_registers(client).await?;

        Ok(new)
    }
}

#[async_trait]
/// Define mapping between Modbus holding registers and the Self type
/// to create and update the type instance by reading values directly from holding registers, or write the values back to holding registers.
pub trait HoldingRegisterMap
where
    Self: Sized + Default,
{
    async fn update_from_holding_registers(
        &mut self,
        client: &mut dyn Reader,
    ) -> Result<(), std::io::Error>;

    async fn from_holding_registers(client: &mut dyn Reader) -> Result<Self, std::io::Error> {
        let mut new = Self::default();
        new.update_from_holding_registers(client).await?;

        Ok(new)
    }

    async fn write_to_registers(&self, client: &mut dyn Writer) -> Result<(), std::io::Error>;
}
