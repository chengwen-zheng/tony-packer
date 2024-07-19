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

#[macro_export]
macro_rules! deserialize {
    ($bytes:expr, $ty:ty) => {{
        let archived = unsafe { rkyv::archived_root::<$ty>($bytes) };
        let deserialized: $ty = archived
            .deserialize(&mut rkyv::de::deserializers::SharedDeserializeMap::new())
            .unwrap();

        deserialized
    }};
}

#[macro_export]
macro_rules! serialize {
    ($t:expr) => {{
        let bytes = rkyv::to_bytes::<_, 1024>($t).unwrap();
        bytes.to_vec()
    }};
}
