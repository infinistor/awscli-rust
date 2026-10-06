//! INI 파서, Config, UserData (TESTCore `Util/INIParser.cs`, `Data/Config/*`)

pub mod ini;

pub use ini::{IniError, IniFile, IniSection, IniValue};
