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
    ) -> tokio_modbus::Result<()>;

    async fn from_input_registers(client: &mut dyn Reader) -> tokio_modbus::Result<Self> {
        let mut new = Self::default();
        let _ = new.update_from_input_registers(client).await?;

        Ok(Ok(new))
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
    ) -> tokio_modbus::Result<()>;

    async fn from_holding_registers(client: &mut dyn Reader) -> tokio_modbus::Result<Self> {
        let mut new = Self::default();
        let _ = new.update_from_holding_registers(client).await?;

        Ok(Ok(new))
    }

    async fn write_to_registers(&self, client: &mut dyn Writer) -> tokio_modbus::Result<()>;
}
