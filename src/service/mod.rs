mod desktop;
mod device;

pub mod rpc {
    tonic::include_proto!("mirrorx.device");
    tonic::include_proto!("mirrorx.desktop");
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("proto_file_descriptor");
}

pub use desktop::DesktopService;
pub use device::DeviceService;
