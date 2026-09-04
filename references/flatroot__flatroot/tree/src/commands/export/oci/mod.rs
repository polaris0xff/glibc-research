//! Turns a finished rootfs into a single-layer OCI container image,
//! packed as one tar archive that both Docker and Podman load without a
//! registry.

mod exporter;
mod model;
mod store;

pub use exporter::OciExporter;
