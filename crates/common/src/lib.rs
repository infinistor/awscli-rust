//! 여러 크레이트가 함께 쓰는 .NET 호환 유틸리티.

pub mod dotnet_datetime;
pub mod dotnet_exit;
pub mod dotnet_format;
pub mod dotnet_http;
pub mod dotnet_json;
pub mod dotnet_offset;
pub mod json;

pub use dotnet_datetime::DotnetDateTime;
pub use dotnet_json::{from_web_json, to_dotnet_json, to_web_json};
pub use dotnet_offset::DotnetDateTimeOffset;
