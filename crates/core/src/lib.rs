pub mod cache;
pub mod config;
pub mod context;
pub mod error;
pub mod module;
pub mod plugin;

pub use cache::*;
pub use config::*;
pub use context::*;
pub use error::*;
pub use module::*;
pub use plugin::*;
pub use relative_path;
pub use rkyv;
pub use rkyv_dyn;
pub use rkyv_typename;
pub use wax;
pub mod record;

#[macro_export]
macro_rules! deserialize {
    ($bytes:expr, $ty:ty) => {{
        let bytes = $bytes; // 在 unsafe 块外绑定元变量
        let archived = unsafe {
            // SAFETY: 调用者必须确保 bytes 包含有效的、由 rkyv 序列化的 $ty 类型数据，
            // 且内存对齐正确。
            rkyv::archived_root::<$ty>(bytes)
        };
        archived
            .deserialize(&mut rkyv::de::deserializers::SharedDeserializeMap::new())
            .expect("Deserialization failed")
    }};
}

#[macro_export]
macro_rules! serialize {
    ($t:expr) => {{
        let bytes = rkyv::to_bytes::<_, 1024>($t).unwrap();
        bytes.to_vec()
    }};
}
