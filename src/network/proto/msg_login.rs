use crate::network::proto::opcode::Opcode;
use crate::network::proto::ProtoMessage;
use crate::network::util::{BytesReader, BytesWriter};

#[derive(Debug)]
pub struct DeviceIDDistributionReq {
    pub offer_device_id: String,
}

impl ProtoMessage for DeviceIDDistributionReq {
    fn opcode(&self) -> u16 {
        Opcode::DesktopConnectAskReq.into()
    }

    fn encode(&self, writer: &mut BytesWriter) {
        writer.write_string(&self.offer_device_id)
    }

    fn decode(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        let mut reader = BytesReader::new(&buf);
        self.offer_device_id = reader.read_string()?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct DeviceIDDistributionResp {
    pub agree: bool,
}

impl ProtoMessage for DeviceIDDistributionResp {
    fn opcode(&self) -> u16 {
        Opcode::DesktopConnectAskResp.into()
    }

    fn encode(&self, writer: &mut BytesWriter) {
        writer.write_bool(self.agree)
    }

    fn decode(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        let mut reader = BytesReader::new(&buf);
        self.agree = reader.read_bool()?;
        Ok(())
    }
}
