//! 服务层 - 服务器核心服务
//!
//! # 服务列表
//!
//! - [`CertService`] - mTLS 证书管理
//! - [`ActivationService`] - 边缘激活状态管理
//! - [`HttpsService`] - HTTPS 服务器
//! - [`MessageBusService`] - 消息总线服务
//! - [`ProvisioningService`] - 边缘预配服务
//! - [`CatalogService`] - 产品和分类统一管理（含内存缓存）

pub mod activation;
pub mod catalog_service;
pub mod cert;
pub mod https;
pub mod message_bus;
pub mod provisioning;
pub mod tenant_binding;

pub use activation::ActivationService;
pub use activation::ActivationStatus;
pub use catalog_service::CatalogService;
pub use cert::CertService;
pub use https::HttpsService;
pub use message_bus::MessageBusService;
pub use provisioning::ProvisioningService;
pub use tenant_binding::{Subscription, TenantBinding};
